// SPDX-License-Identifier: AGPL-3.0-only
use std::sync::Arc;

use serde_json::json;
use umwelt_core::domain::*;
use umwelt_core::{
    Clock, EntityWrite, IdGen, PreparedCommit, RecordPage, RecordQuery, Result, Store, StoreCounts,
    TimelineScope, UmweltError, UmweltErrorCode, commit_fingerprint, latest_reviews,
    normalize_vector, request_fingerprint, validate_doc_type, validate_embedding_space,
    validate_text_hash,
};

pub struct Engine<S: Store> {
    pub(crate) store: S,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGen>,
}

impl<S: Store> Engine<S> {
    pub fn new(store: S, clock: Arc<dyn Clock>, ids: Arc<dyn IdGen>) -> Self {
        Self { store, clock, ids }
    }

    pub fn store(&self) -> &S {
        &self.store
    }
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }
    pub fn clock(&self) -> &dyn Clock {
        self.clock.as_ref()
    }
    pub fn ids(&self) -> &dyn IdGen {
        self.ids.as_ref()
    }

    pub fn put_project(&mut self, project: Project) -> Result<()> {
        self.store.create_project(&project)
    }

    pub fn create_project(&mut self, input: CreateProjectInput) -> Result<Project> {
        let project = Project {
            id: input.id.unwrap_or_else(|| self.ids.new_id()),
            name: input.name,
            description: input.description,
            timescale: input.timescale,
            vocabulary_mode: input.vocabulary_mode,
            created_at: self.clock.now_utc(),
            version: 0,
        };
        self.store.create_project(&project)?;
        Ok(project)
    }

    pub fn project(&self, project_id: &str) -> Result<Project> {
        self.store.project(project_id)
    }

    pub fn projects(&self) -> Result<Vec<Project>> {
        self.store.projects()
    }

    pub fn put_entity_type(&mut self, value: EntityType) -> Result<()> {
        self.store.put_entity_type(&value)
    }

    pub fn create_entity_type(&mut self, input: CreateEntityTypeInput) -> Result<EntityType> {
        let value = EntityType {
            id: input.id.unwrap_or_else(|| self.ids.new_id()),
            project_id: input.project_id,
            name: input.name,
            kind: input.kind,
            fields: input.fields,
        };
        self.store.put_entity_type(&value)?;
        Ok(value)
    }

    pub fn entity_types(&self, project_id: &str) -> Result<Vec<EntityType>> {
        self.store.entity_types(project_id)
    }

    /// Makes a world exist: the project and its entity types, entities,
    /// predicates and axes. What is missing is created in one transaction;
    /// what is already there is left untouched, so the same manifest can run
    /// every time the file is opened. Anything that exists with another
    /// definition refuses the whole manifest with `CONFLICT`, and
    /// `details.divergences` names each one: a manifest never overwrites.
    pub fn ensure_world(&mut self, input: EnsureWorldInput) -> Result<EnsureWorldResult> {
        let project_id = input.project.id.clone();
        let stored = match crate::queries::found(self.store.project(&project_id))? {
            None => StoredWorld::default(),
            Some(project) => {
                let mut entities = Vec::new();
                for wanted in &input.entities {
                    if let Some(entity) =
                        crate::queries::found(self.store.entity(&project_id, &wanted.id))?
                    {
                        entities.push(entity);
                    }
                }
                StoredWorld {
                    project: Some(project),
                    entity_types: self.store.entity_types(&project_id)?,
                    entities,
                    predicates: self.store.predicates(&project_id)?,
                    axes: self.store.axes(&project_id)?,
                }
            }
        };
        let (write, divergences) = plan_world(&input, &stored, &self.clock.now_utc())?;
        if !divergences.is_empty() {
            return Err(UmweltError::public(
                UmweltErrorCode::Conflict,
                format!(
                    "The world already exists with {} different definition(s); nothing was changed.",
                    divergences.len()
                ),
            )
            .with_details(json!({ "divergences": divergences })));
        }
        let created = WorldCreated {
            project: write.project.is_some(),
            entity_types: write.entity_types.iter().map(|v| v.id.clone()).collect(),
            entities: write.entities.iter().map(|v| v.id.clone()).collect(),
            predicates: write.predicates.iter().map(|v| v.id.clone()).collect(),
            axes: write.axes.iter().map(|v| v.id.clone()).collect(),
        };
        self.store.write_world(&write)?;
        Ok(EnsureWorldResult {
            project: self.store.project(&project_id)?,
            created,
        })
    }

    pub fn put_entity(&mut self, value: Entity) -> Result<()> {
        self.store.put_entity(&value)
    }

    pub fn create_entity(&mut self, input: CreateEntityInput) -> Result<Entity> {
        let value = Entity {
            id: input.id.unwrap_or_else(|| self.ids.new_id()),
            project_id: input.project_id,
            type_id: input.type_id,
            name: input.name,
            aliases: input.aliases,
            fields: input.fields,
            external_ref: input.external_ref,
            created_at: input.created_at,
            archived: false,
        };
        self.store.put_entity(&value)?;
        Ok(value)
    }

    /// Registers or updates entities owned by another system, keyed by their
    /// `externalRef`, in one transaction. Ensuring the same input twice changes
    /// nothing.
    pub fn ensure_entities(
        &mut self,
        input: EnsureEntitiesInput,
    ) -> Result<Vec<EnsureEntityResult>> {
        self.store.project(&input.project_id)?;
        let mut seen = std::collections::BTreeSet::new();
        let mut writes = Vec::with_capacity(input.entities.len());
        let mut results = Vec::with_capacity(input.entities.len());
        for item in input.entities {
            if !seen.insert((
                item.external_ref.system.clone(),
                item.external_ref.id.clone(),
            )) {
                return Err(UmweltError::validation(format!(
                    "External reference {}:{} appears twice in the batch.",
                    item.external_ref.system, item.external_ref.id
                )));
            }
            match self
                .store
                .entity_by_external(&input.project_id, &item.external_ref)?
            {
                Some(mut entity) => {
                    if entity.type_id != item.type_id {
                        return Err(UmweltError::validation(format!(
                            "Entity '{}' has type '{}', not '{}'.",
                            entity.id, entity.type_id, item.type_id
                        )));
                    }
                    if item.id.as_ref().is_some_and(|id| *id != entity.id) {
                        return Err(UmweltError::public(
                            UmweltErrorCode::Conflict,
                            format!(
                                "External reference {}:{} already belongs to entity '{}'.",
                                item.external_ref.system, item.external_ref.id, entity.id
                            ),
                        ));
                    }
                    let mut changed = entity.name != item.name;
                    entity.name = item.name;
                    if let Some(aliases) = item.aliases {
                        changed |= entity.aliases != aliases;
                        entity.aliases = aliases;
                    }
                    if let Some(fields) = item.fields {
                        changed |= entity.fields != fields;
                        entity.fields = fields;
                    }
                    if changed {
                        writes.push(EntityWrite::Update(entity.clone()));
                    }
                    results.push(EnsureEntityResult {
                        entity,
                        created: false,
                    });
                }
                None => {
                    let entity = Entity {
                        id: item.id.unwrap_or_else(|| self.ids.new_id()),
                        project_id: input.project_id.clone(),
                        type_id: item.type_id,
                        name: item.name,
                        aliases: item.aliases.unwrap_or_default(),
                        fields: item.fields.unwrap_or_default(),
                        external_ref: Some(item.external_ref),
                        created_at: item.created_at.unwrap_or_default(),
                        archived: false,
                    };
                    writes.push(EntityWrite::Create(entity.clone()));
                    results.push(EnsureEntityResult {
                        entity,
                        created: true,
                    });
                }
            }
        }
        self.store.write_entities(&writes)?;
        Ok(results)
    }

    pub fn resolve_entity(&self, project_id: &str, reference: &ExternalRef) -> Result<Entity> {
        self.store
            .entity_by_external(project_id, reference)?
            .ok_or_else(|| {
                UmweltError::not_found(format!(
                    "No entity has external reference {}:{}.",
                    reference.system, reference.id
                ))
            })
    }

    /// Maps external references to entity ids; unknown references map to none.
    pub fn resolve_entities(
        &self,
        project_id: &str,
        references: Vec<ExternalRef>,
    ) -> Result<Vec<ResolvedEntity>> {
        self.store.project(project_id)?;
        references
            .into_iter()
            .map(|external_ref| {
                let entity_id = self
                    .store
                    .entity_by_external(project_id, &external_ref)?
                    .map(|entity| entity.id);
                Ok(ResolvedEntity {
                    external_ref,
                    entity_id,
                })
            })
            .collect()
    }

    pub fn replace_entity(&mut self, value: Entity) -> Result<()> {
        self.store.update_entity(&value)
    }

    pub fn update_entity(&mut self, input: UpdateEntityInput) -> Result<Entity> {
        let mut value = self.store.entity(&input.project_id, &input.entity_id)?;
        if let Some(name) = input.name {
            value.name = name;
        }
        if let Some(aliases) = input.aliases {
            value.aliases = aliases;
        }
        if let Some(fields) = input.fields {
            value.fields = fields;
        }
        if let Some(external_ref) = input.external_ref {
            value.external_ref = Some(external_ref);
        }
        self.store.update_entity(&value)?;
        Ok(value)
    }

    pub fn archive_entity(&mut self, project_id: &str, entity_id: &str) -> Result<Entity> {
        let mut entity = self.store.entity(project_id, entity_id)?;
        entity.archived = true;
        self.store.update_entity(&entity)?;
        Ok(entity)
    }

    pub fn put_predicate(&mut self, project_id: &str, value: PredicateDef) -> Result<()> {
        self.store.put_predicate(project_id, &value)
    }

    pub fn define_predicate(&mut self, input: DefinePredicateInput) -> Result<PredicateDef> {
        if input.predicate.id.starts_with("umwelt:") {
            return Err(UmweltError::validation(
                "Predicate ids beginning with 'umwelt:' are reserved.",
            ));
        }
        self.store
            .put_predicate(&input.project_id, &input.predicate)?;
        Ok(input.predicate)
    }

    /// Defines an additional time axis. Defining the same axis again is a
    /// no-op; the same id with another label is a `CONFLICT`. Axes belong to
    /// the project: every timeline sees a definition at once.
    pub fn define_axis(&mut self, input: DefineAxisInput) -> Result<AxisDef> {
        validate_axis_id(&input.axis.id)?;
        if input.axis.label.trim().is_empty() {
            return Err(UmweltError::validation("Axis labels cannot be empty."));
        }
        self.store.define_axis(&input.project_id, &input.axis)
    }

    pub fn axes(&self, project_id: &str) -> Result<Vec<AxisDef>> {
        self.store.axes(project_id)
    }

    /// Declares the vectors of one client embedding model. The same
    /// definition again is a no-op; another definition for the id is a
    /// `CONFLICT`. The engine never runs the model.
    pub fn define_embedding_space(
        &mut self,
        input: DefineEmbeddingSpaceInput,
    ) -> Result<EmbeddingSpace> {
        validate_embedding_space(&input.space)?;
        self.store
            .define_embedding_space(&input.project_id, &input.space)
    }

    pub fn embedding_spaces(&self, project_id: &str) -> Result<Vec<EmbeddingSpace>> {
        self.store.embedding_spaces(project_id)
    }

    /// Removes a space and every vector in it. Vectors are derived data; no
    /// history is touched.
    pub fn drop_embedding_space(
        &mut self,
        project_id: &str,
        space_id: &str,
    ) -> Result<DropEmbeddingSpaceResult> {
        let removed = self.store.drop_embedding_space(project_id, space_id)?;
        Ok(DropEmbeddingSpaceResult { removed })
    }

    /// Stores or replaces vectors made by the client's model, in one
    /// transaction. Each vector is checked against the space and normalized.
    pub fn put_embeddings(&mut self, input: PutEmbeddingsInput) -> Result<PutEmbeddingsResult> {
        let space = self.embedding_space(&input.project_id, &input.space_id)?;
        let mut seen = std::collections::BTreeSet::new();
        let mut items = Vec::with_capacity(input.items.len());
        for item in input.items {
            validate_doc_type(&item.doc_type)?;
            validate_text_hash(&item.text_hash)?;
            if !seen.insert((item.doc_type.clone(), item.doc_id.clone())) {
                return Err(UmweltError::validation(format!(
                    "{} '{}' appears twice in the batch.",
                    item.doc_type, item.doc_id
                )));
            }
            let vector = normalize_vector(&item.vector, space.dimensions)?;
            items.push(EmbeddingItem { vector, ..item });
        }
        self.store
            .put_embeddings(&input.project_id, &input.space_id, &items)
    }

    pub fn delete_embeddings(
        &mut self,
        input: DeleteEmbeddingsInput,
    ) -> Result<DeleteEmbeddingsResult> {
        for item in &input.items {
            validate_doc_type(&item.doc_type)?;
        }
        let removed =
            self.store
                .delete_embeddings(&input.project_id, &input.space_id, &input.items)?;
        Ok(DeleteEmbeddingsResult { removed })
    }

    pub(crate) fn embedding_space(
        &self,
        project_id: &str,
        space_id: &str,
    ) -> Result<EmbeddingSpace> {
        self.store
            .embedding_spaces(project_id)?
            .into_iter()
            .find(|space| space.id == space_id)
            .ok_or_else(|| {
                UmweltError::not_found(format!("Embedding space '{space_id}' does not exist."))
            })
    }

    pub fn put_evidence(&mut self, value: EvidenceSource) -> Result<()> {
        self.store.put_evidence(&value)
    }

    pub fn evidence(&self, project_id: &str, evidence_id: &str) -> Result<EvidenceSource> {
        self.store.evidence(project_id, evidence_id)
    }

    pub fn commit(&mut self, input: CommitInput) -> Result<CommitOutcome> {
        let prepared = self.prepare_commit(input)?;
        self.commit_prepared(prepared)
    }

    /// Applies every commit in one transaction. All commits must target the
    /// same timeline.
    pub fn commit_batch(&mut self, inputs: Vec<CommitInput>) -> Result<Vec<CommitOutcome>> {
        if let Some(first) = inputs.first() {
            let timeline = first.timeline();
            if inputs.iter().any(|input| input.timeline() != timeline) {
                return Err(UmweltError::validation(
                    "Every commit of a batch must target the same timeline.",
                ));
            }
        }
        let prepared = inputs
            .into_iter()
            .map(|input| self.prepare_commit(input))
            .collect::<Result<Vec<_>>>()?;
        self.store.commit_batch(&prepared)
    }

    /// Opens a draft from the current head of main or of an active draft
    /// (`base_timeline_id`), at most `MAX_DRAFT_DEPTH` drafts below main.
    /// Writes to the draft are invisible outside it until it is promoted.
    pub fn open_draft(&mut self, input: OpenDraftInput) -> Result<Timeline> {
        let parent_id = input
            .base_timeline_id
            .unwrap_or_else(|| MAIN_TIMELINE_ID.to_owned());
        let id = input.id.unwrap_or_else(|| self.ids.new_id());
        if id.trim().is_empty() || id == MAIN_TIMELINE_ID {
            return Err(UmweltError::validation(format!(
                "'{id}' cannot be used as a draft id."
            )));
        }
        let created_at = self.clock.now_utc();
        self.store.open_draft(
            &input.project_id,
            &id,
            &parent_id,
            input.label.as_deref(),
            &created_at,
        )
    }

    /// Moves every row of the draft into its parent in one transaction. Fails
    /// with `VERSION_CONFLICT` if the parent changed since the draft was
    /// opened. The draft's children move to the parent.
    pub fn promote_draft(&mut self, project_id: &str, timeline_id: &str) -> Result<Timeline> {
        let closed_at = self.clock.now_utc();
        self.store
            .promote_draft(project_id, timeline_id, &closed_at)
    }

    /// Archives the draft: it stays readable by its id, outside every other
    /// read, and accepts no more writes. With `purge`, its rows are removed.
    pub fn discard_draft(
        &mut self,
        project_id: &str,
        timeline_id: &str,
        purge: bool,
    ) -> Result<Timeline> {
        let closed_at = self.clock.now_utc();
        self.store
            .discard_draft(project_id, timeline_id, &closed_at, purge)
    }

    /// Physically removes a discarded draft's rows. Purging again is a no-op.
    pub fn purge_draft(&mut self, project_id: &str, timeline_id: &str) -> Result<PurgeResult> {
        let purged_at = self.clock.now_utc();
        let removed = self
            .store
            .purge_draft(project_id, timeline_id, &purged_at)?;
        Ok(PurgeResult {
            purged: true,
            removed,
        })
    }

    pub fn get_timeline(&self, project_id: &str, timeline_id: &str) -> Result<Timeline> {
        self.store.timeline(project_id, timeline_id)
    }

    pub fn timelines(
        &self,
        project_id: &str,
        status: Option<TimelineStatus>,
    ) -> Result<Vec<Timeline>> {
        self.store.timelines(project_id, status)
    }

    /// A record of main. Use `in_timeline(..).record` to read a draft.
    pub fn record(&self, project_id: &str, record_id: &str) -> Result<Record> {
        self.store
            .record(project_id, &TimelineScope::main(), record_id)
    }

    pub fn event(&self, project_id: &str, event_id: &str) -> Result<Event> {
        self.store
            .event(project_id, &TimelineScope::main(), event_id)
    }

    pub fn transition(&self, project_id: &str, transition_id: &str) -> Result<Transition> {
        self.store.transition(project_id, transition_id)
    }

    pub fn record_state(
        &self,
        project_id: &str,
        record_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<RecordState> {
        self.store.record_state(
            project_id,
            &TimelineScope::main(),
            record_id,
            tick,
            as_of_version,
        )
    }

    pub fn holder_projection(
        &self,
        project_id: &str,
        holder: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Vec<Record>> {
        Ok(self
            .store
            .records_at(
                project_id,
                &TimelineScope::main(),
                Some(holder),
                tick,
                as_of_version,
            )?
            .into_iter()
            .filter(|state| state.transition.is_none())
            .map(|state| state.record)
            .collect())
    }

    pub fn active_records(
        &self,
        project_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Vec<Record>> {
        Ok(self
            .store
            .records_at(
                project_id,
                &TimelineScope::main(),
                None,
                tick,
                as_of_version,
            )?
            .into_iter()
            .filter(|state| state.transition.is_none())
            .map(|state| state.record)
            .collect())
    }

    pub fn query_records(&self, query: &RecordQuery) -> Result<RecordPage> {
        self.store.query_records(query)
    }

    pub fn propose(&mut self, mut input: ProposeInput) -> Result<Proposal> {
        self.store.project(&input.project_id)?;
        input.normalize();
        let proposal = Proposal {
            id: self.ids.new_id(),
            status: ProposalStatus::Pending,
            rejection_rationale: None,
            input,
            reviews: Vec::new(),
        };
        self.store.insert_proposal(&proposal)?;
        Ok(proposal)
    }

    pub fn proposal(&self, proposal_id: &str) -> Result<Proposal> {
        self.store.proposal(proposal_id)
    }

    pub fn proposals(&self, project_id: &str) -> Result<Vec<Proposal>> {
        self.store.proposals(project_id)
    }

    pub fn checkpoint(&mut self) -> Result<()> {
        self.store.checkpoint()
    }

    pub fn review(&mut self, proposal_id: &str, reviews: Vec<ReviewInput>) -> Result<Proposal> {
        let proposal = self.store.proposal(proposal_id)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(UmweltError::public(
                UmweltErrorCode::InvalidState,
                "Only pending proposals can be reviewed.",
            ));
        }
        let operation_count = proposal.input.records.len() + proposal.input.transitions.len();
        let mut stored = Vec::with_capacity(reviews.len());
        for review in reviews {
            if review.operation_index >= operation_count {
                return Err(UmweltError::validation(format!(
                    "Proposal operation index {} is out of range.",
                    review.operation_index
                )));
            }
            stored.push(ProposalReview {
                id: self.ids.new_id(),
                operation_index: review.operation_index,
                decision: review.decision,
                rationale: review.rationale,
                created_at: self.clock.now_utc(),
            });
        }
        self.store.append_reviews(proposal_id, &stored)
    }

    pub fn apply(&mut self, proposal_id: &str, attempt_id: String) -> Result<CommitOutcome> {
        let proposal = self.store.proposal(proposal_id)?;
        let latest = latest_reviews(&proposal.reviews);
        let record_count = proposal.input.records.len();
        let records = proposal
            .input
            .records
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                latest
                    .get(index)
                    .is_some_and(|review| review.decision == ReviewDecision::Accepted)
            })
            .map(|(_, record)| record.clone())
            .collect::<Vec<_>>();
        let transitions = proposal
            .input
            .transitions
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                latest
                    .get(&(record_count + index))
                    .is_some_and(|review| review.decision == ReviewDecision::Accepted)
            })
            .map(|(_, transition)| transition.clone())
            .collect::<Vec<_>>();
        let accepted = records.len() + transitions.len();
        let total = record_count + proposal.input.transitions.len();
        let prepared = self.prepare_commit(CommitInput {
            project_id: proposal.input.project_id,
            attempt_id,
            timeline_id: proposal.input.timeline_id,
            expected_version: proposal.input.expected_version,
            event: proposal.input.event,
            records,
            transitions,
            preconditions: Vec::new(),
        })?;
        let status = if accepted == total {
            ProposalStatus::Applied
        } else {
            ProposalStatus::PartiallyApplied
        };
        self.store.apply_proposal(&prepared, proposal_id, status)
    }

    pub fn reject(&mut self, proposal_id: &str, rationale: &str) -> Result<Proposal> {
        let proposal = self.store.proposal(proposal_id)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(UmweltError::public(
                UmweltErrorCode::InvalidState,
                "Only pending proposals can be rejected.",
            ));
        }
        self.store
            .set_proposal_status(proposal_id, ProposalStatus::Rejected, Some(rationale))
    }

    pub fn transmit(&mut self, input: TransmitInput) -> Result<CommitOutcome> {
        let prepared = self.prepare_transmit(input)?;
        self.commit_prepared(prepared)
    }

    /// Applies every transmit in one transaction. All must target the same
    /// timeline.
    pub fn transmit_batch(&mut self, inputs: Vec<TransmitInput>) -> Result<Vec<CommitOutcome>> {
        if let Some(first) = inputs.first() {
            let timeline = first.timeline_id.as_deref().unwrap_or(MAIN_TIMELINE_ID);
            if inputs
                .iter()
                .any(|input| input.timeline_id.as_deref().unwrap_or(MAIN_TIMELINE_ID) != timeline)
            {
                return Err(UmweltError::validation(
                    "Every transmit of a batch must target the same timeline.",
                ));
            }
        }
        let prepared = inputs
            .into_iter()
            .map(|input| self.prepare_transmit(input))
            .collect::<Result<Vec<_>>>()?;
        self.store.commit_batch(&prepared)
    }

    /// A batch whose items leave what they have in common to `shared`. Each
    /// item is completed first, so it is fingerprinted and applied exactly as
    /// if it had been written out in full.
    pub fn transmit_batch_shared(
        &mut self,
        shared: &TransmitShared,
        items: Vec<TransmitBatchItem>,
    ) -> Result<Vec<CommitOutcome>> {
        let inputs = items
            .into_iter()
            .map(|item| item.complete(shared))
            .collect::<Result<Vec<_>>>()?;
        self.transmit_batch(inputs)
    }

    /// A transmit is fingerprinted from the request, which is a fraction of
    /// the size of the commit it becomes.
    fn prepare_transmit(&self, input: TransmitInput) -> Result<PreparedCommit> {
        let fingerprint = request_fingerprint("transmit", &input)?;
        let commit = self.transmit_commit(input);
        self.prepare_with_fingerprint(commit, fingerprint)
    }

    /// Records what every witness saw at one event, in one atomic commit.
    pub fn witness(&mut self, input: WitnessInput) -> Result<CommitOutcome> {
        let fingerprint = request_fingerprint("witness", &input)?;
        if input.claims.is_empty() || input.witnesses.is_empty() {
            return Err(UmweltError::validation(
                "witness needs at least one claim and one witness.",
            ));
        }
        let mut holders = std::collections::BTreeSet::new();
        for witness in &input.witnesses {
            if !holders.insert(witness.holder.as_str()) {
                return Err(UmweltError::validation(format!(
                    "Witness '{}' appears twice.",
                    witness.holder
                )));
            }
            if !witness.record_ids.is_empty() && witness.record_ids.len() != input.claims.len() {
                return Err(UmweltError::validation(format!(
                    "Witness '{}' must give one record id per claim or none.",
                    witness.holder
                )));
            }
        }
        let mut event = input.event;
        let event_id = event.id.clone().unwrap_or_else(|| self.ids.new_id());
        event.id = Some(event_id.clone());
        let position = event.position;
        let provenance = event.provenance.clone();
        let common = |id: Option<String>| RecordInputCommon {
            id,
            position: Some(position),
            provenance: Some(provenance.clone()),
        };
        let mut records = input
            .claims
            .iter()
            .filter(|witnessed| witnessed.canon)
            .map(|witnessed| RecordInput::Fact {
                common: common(witnessed.fact_id.clone()),
                claim: witnessed.claim.clone(),
            })
            .collect::<Vec<_>>();
        for witness in &input.witnesses {
            for (index, witnessed) in input.claims.iter().enumerate() {
                records.push(RecordInput::Holding {
                    common: common(witness.record_ids.get(index).cloned()),
                    holder: witness.holder.clone(),
                    attitude: witness.attitude.clone(),
                    claim: witnessed.claim.clone(),
                    acquisition: Acquisition::Witnessed {
                        event_id: event_id.clone(),
                    },
                    confidence: witness.confidence,
                    accessibility: witness.accessibility.clone(),
                    text: None,
                    secrecy: None,
                });
            }
        }
        let prepared = self.prepare_with_fingerprint(
            CommitInput {
                project_id: input.project_id,
                attempt_id: input.attempt_id,
                timeline_id: input.timeline_id,
                expected_version: input.expected_version,
                event,
                records,
                transitions: Vec::new(),
                preconditions: Vec::new(),
            },
            fingerprint,
        )?;
        self.commit_prepared(prepared)
    }

    /// Supersedes many active Holdings with adjusted copies in one commit.
    pub fn adjust_holdings(&mut self, input: AdjustHoldingsInput) -> Result<CommitOutcome> {
        let fingerprint = request_fingerprint("holding.adjustBatch", &input)?;
        if input.adjustments.is_empty() {
            return Err(UmweltError::validation(
                "adjustBatch needs at least one adjustment.",
            ));
        }
        let timeline = input.timeline_id.as_deref().unwrap_or(MAIN_TIMELINE_ID);
        let provenance = system_provenance(input.provenance, "umwelt:adjust");
        let mut seen = std::collections::BTreeSet::new();
        let mut records = Vec::with_capacity(input.adjustments.len());
        let mut transitions = Vec::with_capacity(input.adjustments.len());
        let mut preconditions = Vec::with_capacity(input.adjustments.len());
        for adjustment in input.adjustments {
            if !seen.insert(adjustment.record_id.clone()) {
                return Err(UmweltError::validation(format!(
                    "Holding '{}' is adjusted twice.",
                    adjustment.record_id
                )));
            }
            if adjustment.confidence.is_none()
                && adjustment.accessibility.is_none()
                && adjustment.attitude.is_none()
            {
                return Err(UmweltError::validation(format!(
                    "Adjustment of '{}' changes nothing.",
                    adjustment.record_id
                )));
            }
            let Record::Holding {
                holder,
                attitude,
                claim,
                acquisition,
                confidence,
                accessibility,
                text,
                secrecy,
                ..
            } = self
                .in_timeline(timeline)
                .record(&input.project_id, &adjustment.record_id)?
            else {
                return Err(UmweltError::validation(format!(
                    "Record '{}' is not a Holding.",
                    adjustment.record_id
                )));
            };
            let new_id = adjustment
                .new_record_id
                .unwrap_or_else(|| self.ids.new_id());
            preconditions.push(Precondition::ActiveHolding {
                record_id: adjustment.record_id.clone(),
                holder: holder.clone(),
                at: input.position,
                copy_claim_to_operation: None,
            });
            records.push(RecordInput::Holding {
                common: RecordInputCommon {
                    id: Some(new_id.clone()),
                    position: Some(input.position),
                    provenance: Some(provenance.clone()),
                },
                holder,
                attitude: adjustment.attitude.unwrap_or(attitude),
                claim,
                acquisition,
                confidence: adjustment.confidence.unwrap_or(confidence),
                accessibility: adjustment.accessibility.unwrap_or(accessibility),
                text,
                secrecy,
            });
            transitions.push(TransitionInput {
                id: None,
                target_record_id: adjustment.record_id,
                status: TransitionStatus::Superseded,
                superseded_by: Some(new_id),
                position: Some(input.position),
                provenance: Some(provenance.clone()),
            });
        }
        let commit = CommitInput {
            project_id: input.project_id,
            attempt_id: input.attempt_id,
            timeline_id: input.timeline_id,
            expected_version: input.expected_version,
            event: system_event(
                input.position,
                input.on,
                provenance,
                "umwelt:adjust",
                "Adjustment",
            ),
            records,
            transitions,
            preconditions,
        };
        let prepared = self.prepare_with_fingerprint(commit, fingerprint)?;
        self.commit_prepared(prepared)
    }

    /// Applies one status to many records in one commit.
    pub fn transition_batch(&mut self, input: TransitionBatchInput) -> Result<CommitOutcome> {
        let fingerprint = request_fingerprint("transition.batch", &input)?;
        if input.record_ids.is_empty() {
            return Err(UmweltError::validation(
                "transition.batch needs at least one record.",
            ));
        }
        // `superseded` cannot be asked for: `BatchTransitionStatus` has no such
        // value, so the request is refused when it is read.
        let status = TransitionStatus::from(input.status);
        let provenance = system_provenance(input.provenance, "umwelt:transition");
        let transitions = input
            .record_ids
            .into_iter()
            .map(|record_id| TransitionInput {
                id: None,
                target_record_id: record_id,
                status: status.clone(),
                superseded_by: None,
                position: Some(input.position),
                provenance: Some(provenance.clone()),
            })
            .collect();
        let commit = CommitInput {
            project_id: input.project_id,
            attempt_id: input.attempt_id,
            timeline_id: input.timeline_id,
            expected_version: input.expected_version,
            event: system_event(
                input.position,
                input.on,
                provenance,
                "umwelt:transition",
                "Transition",
            ),
            records: Vec::new(),
            transitions,
            preconditions: Vec::new(),
        };
        let prepared = self.prepare_with_fingerprint(commit, fingerprint)?;
        self.commit_prepared(prepared)
    }

    /// Records many Disposition values in one commit.
    pub fn set_dispositions(&mut self, input: SetDispositionsInput) -> Result<CommitOutcome> {
        let fingerprint = request_fingerprint("disposition.setBatch", &input)?;
        if input.dispositions.is_empty() {
            return Err(UmweltError::validation(
                "disposition.setBatch needs at least one disposition.",
            ));
        }
        let provenance = system_provenance(input.provenance, "umwelt:disposition");
        let records = input
            .dispositions
            .into_iter()
            .map(|setting| RecordInput::Disposition {
                common: RecordInputCommon {
                    id: setting.id,
                    position: Some(input.position),
                    provenance: Some(provenance.clone()),
                },
                holder: setting.holder,
                target: setting.target,
                dimension: setting.dimension,
                value: setting.value,
            })
            .collect();
        let commit = CommitInput {
            project_id: input.project_id,
            attempt_id: input.attempt_id,
            timeline_id: input.timeline_id,
            expected_version: input.expected_version,
            event: system_event(
                input.position,
                input.on,
                provenance,
                "umwelt:disposition",
                "Dispositions",
            ),
            records,
            transitions: Vec::new(),
            preconditions: Vec::new(),
        };
        let prepared = self.prepare_with_fingerprint(commit, fingerprint)?;
        self.commit_prepared(prepared)
    }

    /// Replays an active draft on top of its parent's current head so it can
    /// be promoted. All of its commits apply again, or nothing changes and the
    /// error names the commit that no longer fits. Refused while drafts opened
    /// from it still depend on it.
    pub fn rebase_draft(&mut self, project_id: &str, timeline_id: &str) -> Result<Timeline> {
        self.store.rebase_draft(project_id, timeline_id)
    }

    /// Checks every operation of a pending proposal against the current head
    /// of its timeline, without applying anything. Operations are checked in
    /// order, each together with the earlier ones that are still valid. A
    /// proposal with an `expectedVersion` is moved to the current head.
    pub fn rebase_proposal(&mut self, proposal_id: &str) -> Result<ProposalRebaseResult> {
        let mut proposal = self.store.proposal(proposal_id)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(UmweltError::public(
                UmweltErrorCode::InvalidState,
                "Only pending proposals can be rebased.",
            ));
        }
        let input = &proposal.input;
        let head_version = self
            .store
            .timeline(
                &input.project_id,
                input.timeline_id.as_deref().unwrap_or(MAIN_TIMELINE_ID),
            )?
            .head_version;
        let record_count = input.records.len();
        let operation_count = record_count + input.transitions.len();
        let mut accepted: Vec<usize> = Vec::new();
        let mut operations = Vec::with_capacity(operation_count);
        for index in 0..operation_count {
            let candidate = accepted
                .iter()
                .copied()
                .chain(std::iter::once(index))
                .collect::<Vec<_>>();
            let commit = CommitInput {
                project_id: input.project_id.clone(),
                attempt_id: format!("rebase-check:{proposal_id}"),
                timeline_id: input.timeline_id.clone(),
                expected_version: None,
                event: input.event.clone(),
                records: candidate
                    .iter()
                    .filter(|operation| **operation < record_count)
                    .map(|operation| input.records[*operation].clone())
                    .collect(),
                transitions: candidate
                    .iter()
                    .filter(|operation| **operation >= record_count)
                    .map(|operation| input.transitions[*operation - record_count].clone())
                    .collect(),
                preconditions: Vec::new(),
            };
            let checked = self
                .prepare_commit(commit)
                .and_then(|prepared| self.store.check_commit(&prepared));
            match checked {
                Ok(_) => {
                    accepted.push(index);
                    operations.push(OperationCheck {
                        operation_index: index,
                        valid: true,
                        error: None,
                    });
                }
                Err(error) => operations.push(OperationCheck {
                    operation_index: index,
                    valid: false,
                    error: Some(OperationError {
                        code: serde_json::to_value(error.code)
                            .ok()
                            .and_then(|value| value.as_str().map(str::to_owned))
                            .unwrap_or_default(),
                        message: error.message,
                    }),
                }),
            }
        }
        if proposal.input.expected_version.is_some() {
            proposal.input.expected_version = Some(head_version);
            self.store.replace_proposal(&proposal)?;
        }
        Ok(ProposalRebaseResult {
            proposal,
            head_version,
            operations,
        })
    }

    fn commit_prepared(&mut self, prepared: PreparedCommit) -> Result<CommitOutcome> {
        let mut outcomes = self.store.commit_batch(&[prepared])?;
        outcomes.pop().ok_or_else(|| {
            UmweltError::public(UmweltErrorCode::InvalidState, "Commit returned no outcome.")
        })
    }

    pub fn counts(&self, project_id: &str) -> Result<StoreCounts> {
        self.store.counts(project_id)
    }

    /// Write defaults are filled before the fingerprint, so a retry that
    /// spells out a field the first request omitted is the same attempt.
    fn prepare_commit(&self, mut input: CommitInput) -> Result<PreparedCommit> {
        input.normalize();
        let fingerprint = commit_fingerprint(&input)?;
        self.prepare_with_fingerprint(input, fingerprint)
    }

    fn prepare_with_fingerprint(
        &self,
        input: CommitInput,
        fingerprint: String,
    ) -> Result<PreparedCommit> {
        if input.timeline_id.as_deref().is_some_and(str::is_empty) {
            return Err(UmweltError::validation("timelineId cannot be empty."));
        }
        let event_id = input.event.id.clone().unwrap_or_else(|| self.ids.new_id());
        let record_ids = input
            .records
            .iter()
            .map(|record| {
                record
                    .common()
                    .id
                    .clone()
                    .unwrap_or_else(|| self.ids.new_id())
            })
            .collect();
        let transition_ids = input
            .transitions
            .iter()
            .map(|transition| transition.id.clone().unwrap_or_else(|| self.ids.new_id()))
            .collect();
        Ok(PreparedCommit {
            input,
            fingerprint,
            event_id,
            record_ids,
            transition_ids,
        })
    }

    fn transmit_commit(&self, input: TransmitInput) -> CommitInput {
        let copy_source_claim = input.distorted_claim.is_none();
        let claim = input.distorted_claim.unwrap_or_else(|| Claim {
            subject: String::new(),
            predicate: String::new(),
            object: ClaimObject::None,
            qualifiers: std::collections::BTreeMap::new(),
            polarity: true,
            about: None,
        });
        let provenance = Provenance {
            origin: Origin::System {
                name: "umwelt:transmit".to_owned(),
            },
            reason: None,
            metadata: None,
        };
        CommitInput {
            project_id: input.project_id,
            attempt_id: input.attempt_id,
            timeline_id: input.timeline_id,
            expected_version: None,
            event: EventInput {
                id: None,
                event_type: "umwelt:transmit".to_owned(),
                title: "Transmission".to_owned(),
                description: None,
                position: input.position,
                on: input.on,
                participants: vec![input.from.clone(), input.to.clone()],
                location: None,
                evidence: Vec::new(),
                provenance: provenance.clone(),
            },
            records: vec![RecordInput::Holding {
                common: RecordInputCommon {
                    id: input.record_id,
                    position: Some(input.position),
                    provenance: Some(provenance),
                },
                holder: input.to,
                attitude: input.attitude,
                claim,
                acquisition: Acquisition::ToldBy {
                    source_holder_id: input.from.clone(),
                    source_record_id: Some(input.source_record_id.clone()),
                    intent: input.intent,
                },
                confidence: input.confidence,
                accessibility: Accessibility::Available,
                text: input.text,
                secrecy: None,
            }],
            transitions: Vec::new(),
            preconditions: vec![Precondition::ActiveHolding {
                record_id: input.source_record_id,
                holder: input.from,
                at: input.position,
                copy_claim_to_operation: copy_source_claim.then_some(0),
            }],
        }
    }
}

/// The client's provenance, or one naming the engine operation that wrote it.
fn system_provenance(provenance: Option<Provenance>, name: &str) -> Provenance {
    provenance.unwrap_or_else(|| Provenance {
        origin: Origin::System {
            name: name.to_owned(),
        },
        reason: None,
        metadata: None,
    })
}

/// Events the engine writes for its batch operations. Their `umwelt:` type
/// keeps them out of full-text search.
fn system_event(
    position: Position,
    on: AxisPositions,
    provenance: Provenance,
    event_type: &str,
    title: &str,
) -> EventInput {
    EventInput {
        id: None,
        event_type: event_type.to_owned(),
        title: title.to_owned(),
        description: None,
        position,
        on,
        participants: Vec::new(),
        location: None,
        evidence: Vec::new(),
        provenance,
    }
}
