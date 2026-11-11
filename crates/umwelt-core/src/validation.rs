// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::{BTreeMap, BTreeSet};

use crate::domain::*;
use crate::{PreparedCommit, Result, UmweltError, UmweltErrorCode};

pub trait ValidationView {
    fn project(&self, project_id: &str) -> Result<Option<Project>>;
    fn entity_kind(&self, project_id: &str, entity_id: &str) -> Result<Option<EntityKind>>;
    fn predicate(&self, project_id: &str, predicate_id: &str) -> Result<Option<PredicateDef>>;
    fn record(&self, project_id: &str, record_id: &str) -> Result<Option<Record>>;
    fn record_state(
        &self,
        project_id: &str,
        record_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Option<RecordState>>;
    fn event_exists(&self, project_id: &str, event_id: &str) -> Result<bool>;
    fn operation_id_exists(&self, project_id: &str, id: &str) -> Result<bool>;
    /// Whether the project defines the additional axis `axis_id`.
    fn axis_exists(&self, project_id: &str, axis_id: &str) -> Result<bool>;

    /// The version of the timeline the commit writes to. Preconditions are
    /// checked there. Main's head is the project version.
    fn head_version(&self, project: &Project) -> Result<Version> {
        Ok(project.version)
    }

    /// Who a secret Holding may be shared with; `None` when it is no secret.
    /// Stores should answer without decoding the record.
    fn secret_shared_with(&self, project_id: &str, record_id: &str) -> Result<Option<Vec<String>>> {
        Ok(match self.record(project_id, record_id)? {
            Some(Record::Holding {
                secrecy: Some(secrecy),
                ..
            }) => Some(secrecy.shared_with),
            _ => None,
        })
    }

    /// Whether a record exists. Stores should answer without decoding it.
    fn record_exists(&self, project_id: &str, record_id: &str) -> Result<bool> {
        Ok(self.record(project_id, record_id)?.is_some())
    }

    /// A record's tick, if it exists. Stores should answer without decoding it.
    fn record_tick(&self, project_id: &str, record_id: &str) -> Result<Option<i64>> {
        Ok(self
            .record(project_id, record_id)?
            .map(|record| record_common(&record).position.tick))
    }
}

pub struct ValidatedCommit {
    pub warnings: Vec<Warning>,
    pub resolved_claims: BTreeMap<usize, Claim>,
}

pub fn validate_commit<V: ValidationView>(
    view: &V,
    prepared: &PreparedCommit,
) -> Result<ValidatedCommit> {
    let input = &prepared.input;
    if input.attempt_id.trim().is_empty() {
        return Err(UmweltError::validation("attemptId cannot be empty."));
    }
    if input.records.is_empty() && input.transitions.is_empty() {
        return Err(UmweltError::validation(
            "A commit must carry at least one record or transition.",
        ));
    }
    if prepared.record_ids.len() != input.records.len()
        || prepared.transition_ids.len() != input.transitions.len()
    {
        return Err(UmweltError::public(
            UmweltErrorCode::InvalidState,
            "Prepared operation ids do not match the commit input.",
        ));
    }
    input.event.position.validate()?;
    validate_axis_positions(&input.event.on)?;
    let project = view.project(&input.project_id)?.ok_or_else(|| {
        UmweltError::not_found(format!("Project '{}' does not exist.", input.project_id))
    })?;
    let head_version = view.head_version(&project)?;
    let mut resolved_claims = BTreeMap::new();
    let mut known_secrecy: BTreeMap<&str, Option<Vec<String>>> = BTreeMap::new();
    for precondition in &input.preconditions {
        match precondition {
            Precondition::ActiveHolding {
                record_id,
                holder,
                at,
                copy_claim_to_operation,
            } => {
                at.validate()?;
                let state = view
                    .record_state(&input.project_id, record_id, at.tick, head_version)?
                    .ok_or_else(|| {
                        UmweltError::validation(format!(
                            "Holding '{record_id}' is not active at tick {}.",
                            at.tick
                        ))
                    })?;
                if state.transition.is_some() {
                    return Err(UmweltError::validation(format!(
                        "Holding '{record_id}' is not active at tick {}.",
                        at.tick
                    )));
                }
                let (source_holder, claim) = match state.record {
                    Record::Holding {
                        holder,
                        claim,
                        secrecy,
                        ..
                    } => {
                        known_secrecy
                            .insert(record_id.as_str(), secrecy.map(|value| value.shared_with));
                        (holder, claim)
                    }
                    _ => {
                        return Err(UmweltError::validation(format!(
                            "Record '{record_id}' is not a Holding."
                        )));
                    }
                };
                if source_holder != *holder {
                    return Err(UmweltError::validation(format!(
                        "Holding '{record_id}' belongs to a different holder."
                    )));
                }
                if let Some(index) = copy_claim_to_operation {
                    if *index >= input.records.len() {
                        return Err(UmweltError::public(
                            UmweltErrorCode::InvalidState,
                            "Claim copy target is outside the commit records.",
                        ));
                    }
                    resolved_claims.insert(*index, claim);
                }
            }
        }
    }
    ensure_unique_ids(prepared)?;
    for id in std::iter::once(&prepared.event_id)
        .chain(prepared.record_ids.iter())
        .chain(prepared.transition_ids.iter())
    {
        if view.operation_id_exists(&input.project_id, id)? {
            return Err(id_conflict("operation", id));
        }
    }
    for axis in input.event.on.keys() {
        if !view.axis_exists(&input.project_id, axis)? {
            return Err(UmweltError::validation(format!(
                "Axis '{axis}' is not defined in this project."
            )));
        }
    }
    for participant in &input.event.participants {
        require_entity(view, &input.project_id, participant, "event participant")?;
    }
    if let Some(location) = &input.event.location {
        require_entity(view, &input.project_id, location, "event location")?;
    }

    let new_records: BTreeSet<&str> = prepared.record_ids.iter().map(String::as_str).collect();
    let mut warnings = Vec::new();
    for (record_index, record) in input.records.iter().enumerate() {
        record.common().position_in(&input.event).validate()?;
        match record {
            RecordInput::Fact { claim, .. } => {
                validate_claim(view, &project, claim, &new_records, &mut warnings)?;
            }
            RecordInput::Holding {
                holder,
                claim,
                acquisition,
                confidence,
                ..
            } => {
                require_holder(view, &input.project_id, holder)?;
                let claim = resolved_claims.get(&record_index).unwrap_or(claim);
                validate_claim(view, &project, claim, &new_records, &mut warnings)?;
                if !confidence.is_finite() || !(0.0..=1.0).contains(confidence) {
                    return Err(UmweltError::validation(
                        "Holding confidence must be between 0 and 1.",
                    ));
                }
                validate_acquisition(
                    view,
                    &input.project_id,
                    &prepared.event_id,
                    acquisition,
                    &new_records,
                )?;
                if let Acquisition::ToldBy {
                    source_holder_id,
                    source_record_id: Some(source),
                    ..
                } = acquisition
                {
                    let shared_with = match known_secrecy.get(source.as_str()) {
                        Some(known) => known.clone(),
                        None if new_records.contains(source.as_str()) => None,
                        None => view.secret_shared_with(&input.project_id, source)?,
                    };
                    if shared_with.is_some_and(|allowed| {
                        holder != source_holder_id && !allowed.contains(holder)
                    }) {
                        warnings.push(Warning {
                            code: "SECRET_BREACH".to_owned(),
                            message: format!(
                                "'{source_holder_id}' told '{holder}' the secret '{source}'."
                            ),
                        });
                    }
                }
            }
            RecordInput::Awareness { holder, target, .. }
            | RecordInput::Disposition { holder, target, .. } => {
                require_holder(view, &input.project_id, holder)?;
                require_entity(view, &input.project_id, target, "record target")?;
            }
        }
    }

    for transition in &input.transitions {
        let transition_position = transition.position_in(&input.event);
        transition_position.validate()?;
        require_record(
            view,
            &input.project_id,
            &transition.target_record_id,
            &new_records,
        )?;
        let target_tick = record_tick(
            view,
            &input.project_id,
            &transition.target_record_id,
            prepared,
        )?;
        if transition_position.tick < target_tick {
            return Err(UmweltError::validation(
                "Transition position cannot precede its target record.",
            ));
        }
        match transition.status {
            TransitionStatus::Superseded => {
                let replacement = transition.superseded_by.as_deref().ok_or_else(|| {
                    UmweltError::validation(
                        "supersededBy is required when transition status is superseded.",
                    )
                })?;
                require_record(view, &input.project_id, replacement, &new_records)?;
                let replacement_tick = record_tick(view, &input.project_id, replacement, prepared)?;
                if replacement_tick > transition_position.tick {
                    return Err(UmweltError::validation(
                        "A superseding record cannot start after the transition.",
                    ));
                }
            }
            _ if transition.superseded_by.is_some() => {
                return Err(UmweltError::validation(
                    "supersededBy is only allowed when transition status is superseded.",
                ));
            }
            _ => {}
        }
    }
    Ok(ValidatedCommit {
        warnings,
        resolved_claims,
    })
}

pub fn materialize_commit(
    prepared: &PreparedCommit,
    version: Version,
    validated: ValidatedCommit,
) -> (Event, Vec<Record>, Vec<Transition>, CommitOutcome) {
    let input = &prepared.input;
    let event = Event {
        id: prepared.event_id.clone(),
        project_id: input.project_id.clone(),
        timeline_id: input.timeline().to_owned(),
        event_type: input.event.event_type.clone(),
        title: input.event.title.clone(),
        description: input.event.description.clone(),
        position: input.event.position,
        on: input.event.on.clone(),
        participants: input.event.participants.clone(),
        location: input.event.location.clone(),
        evidence: input.event.evidence.clone(),
        provenance: input.event.provenance.clone(),
        recorded_at: version,
    };
    let records = input
        .records
        .iter()
        .zip(&prepared.record_ids)
        .enumerate()
        .map(|(index, (record, id))| {
            materialize_record(
                record,
                id,
                &prepared.event_id,
                input,
                version,
                validated.resolved_claims.get(&index),
            )
        })
        .collect();
    let transitions = input
        .transitions
        .iter()
        .zip(&prepared.transition_ids)
        .enumerate()
        .map(|(ordinal, (transition, id))| Transition {
            id: id.clone(),
            project_id: input.project_id.clone(),
            timeline_id: input.timeline().to_owned(),
            target_record_id: transition.target_record_id.clone(),
            status: transition.status.clone(),
            superseded_by: transition.superseded_by.clone(),
            position: transition.position_in(&input.event),
            recorded_at: version,
            ordinal: ordinal as u32,
            event_id: prepared.event_id.clone(),
            provenance: transition.provenance_in(&input.event).clone(),
        })
        .collect();
    let outcome = CommitOutcome {
        event_id: prepared.event_id.clone(),
        record_ids: prepared.record_ids.clone(),
        transition_ids: prepared.transition_ids.clone(),
        version,
        warnings: validated.warnings,
    };
    (event, records, transitions, outcome)
}

fn materialize_record(
    record: &RecordInput,
    id: &str,
    event_id: &str,
    input: &CommitInput,
    version: Version,
    resolved_claim: Option<&Claim>,
) -> Record {
    let common = |source: &RecordInputCommon| RecordCommon {
        id: id.to_owned(),
        project_id: input.project_id.clone(),
        timeline_id: input.timeline().to_owned(),
        position: source.position_in(&input.event),
        recorded_at: version,
        event_id: event_id.to_owned(),
        provenance: source.provenance_in(&input.event).clone(),
    };
    match record {
        RecordInput::Fact {
            common: source,
            claim,
        } => Record::Fact {
            common: common(source),
            claim: claim.clone(),
        },
        RecordInput::Holding {
            common: source,
            holder,
            attitude,
            claim,
            acquisition,
            confidence,
            accessibility,
            text,
            secrecy,
        } => Record::Holding {
            common: common(source),
            holder: holder.clone(),
            attitude: attitude.clone(),
            claim: resolved_claim.unwrap_or(claim).clone(),
            acquisition: acquisition.clone(),
            confidence: *confidence,
            accessibility: accessibility.clone(),
            text: text.clone(),
            secrecy: secrecy.clone(),
        },
        RecordInput::Awareness {
            common: source,
            holder,
            target,
        } => Record::Awareness {
            common: common(source),
            holder: holder.clone(),
            target: target.clone(),
        },
        RecordInput::Disposition {
            common: source,
            holder,
            target,
            dimension,
            value,
        } => Record::Disposition {
            common: common(source),
            holder: holder.clone(),
            target: target.clone(),
            dimension: dimension.clone(),
            value: value.clone(),
        },
    }
}

fn ensure_unique_ids(prepared: &PreparedCommit) -> Result<()> {
    let mut ids = BTreeSet::new();
    for id in std::iter::once(&prepared.event_id)
        .chain(prepared.record_ids.iter())
        .chain(prepared.transition_ids.iter())
    {
        if id.trim().is_empty() {
            return Err(UmweltError::validation("Provided ids cannot be empty."));
        }
        if !ids.insert(id) {
            return Err(id_conflict("operation", id));
        }
    }
    Ok(())
}

fn validate_claim<V: ValidationView>(
    view: &V,
    project: &Project,
    claim: &Claim,
    new_records: &BTreeSet<&str>,
    warnings: &mut Vec<Warning>,
) -> Result<()> {
    require_entity(view, &project.id, &claim.subject, "claim subject")?;
    if let ClaimObject::Entity { id } = &claim.object {
        require_entity(view, &project.id, id, "claim object")?;
    }
    if let Some(about) = &claim.about {
        require_record(view, &project.id, about, new_records)?;
    }
    validate_reserved_claim(view, &project.id, claim)?;
    match view.predicate(&project.id, &claim.predicate)? {
        Some(predicate) => validate_object_kind(&predicate, &claim.object),
        None if project.vocabulary_mode == VocabularyMode::Strict => Err(UmweltError::validation(
            format!("Predicate '{}' does not exist.", claim.predicate),
        )),
        None => {
            warnings.push(Warning {
                code: "UNKNOWN_PREDICATE".to_owned(),
                message: format!(
                    "Predicate '{}' is not defined in this open-vocabulary project.",
                    claim.predicate
                ),
            });
            Ok(())
        }
    }
}

/// Reserved predicates carry a fixed object shape.
fn validate_reserved_claim<V: ValidationView>(
    view: &V,
    project_id: &str,
    claim: &Claim,
) -> Result<()> {
    match claim.predicate.as_str() {
        NAME_PREDICATE_ID | TITLE_PREDICATE_ID | ALIAS_PREDICATE_ID => match &claim.object {
            ClaimObject::Value {
                value: serde_json::Value::String(text),
            } if !text.trim().is_empty() => Ok(()),
            _ => Err(UmweltError::validation(format!(
                "'{}' needs a non-empty string value.",
                claim.predicate
            ))),
        },
        HOLDS_PREDICATE_ID => {
            require_holder(view, project_id, &claim.subject)?;
            let key = match &claim.object {
                ClaimObject::Value {
                    value: serde_json::Value::Object(fields),
                } if fields.len() == 1 => fields.get("claimKey").and_then(|key| key.as_str()),
                _ => None,
            };
            if key.is_some_and(|key| {
                key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
            }) {
                Ok(())
            } else {
                Err(UmweltError::validation(
                    "'umwelt:holds' needs the value { \"claimKey\": <64 hex characters> }.",
                ))
            }
        }
        _ => Ok(()),
    }
}

fn validate_object_kind(predicate: &PredicateDef, object: &ClaimObject) -> Result<()> {
    let compatible = matches!(
        (&predicate.object, object),
        (ObjectKind::Entity, ClaimObject::Entity { .. })
            | (ObjectKind::Value, ClaimObject::Value { .. })
            | (ObjectKind::None, ClaimObject::None)
    );
    if compatible {
        Ok(())
    } else {
        Err(UmweltError::validation(format!(
            "Claim object is incompatible with predicate '{}'.",
            predicate.id
        )))
    }
}

fn validate_acquisition<V: ValidationView>(
    view: &V,
    project_id: &str,
    current_event_id: &str,
    acquisition: &Acquisition,
    new_records: &BTreeSet<&str>,
) -> Result<()> {
    match acquisition {
        Acquisition::ToldBy {
            source_holder_id,
            source_record_id,
            ..
        } => {
            require_holder(view, project_id, source_holder_id)?;
            if let Some(record_id) = source_record_id {
                require_record(view, project_id, record_id, new_records)?;
            }
        }
        Acquisition::Inferred {
            from_record_ids, ..
        } => {
            for record_id in from_record_ids {
                require_record(view, project_id, record_id, new_records)?;
            }
        }
        Acquisition::Witnessed { event_id } => {
            if event_id != current_event_id && !view.event_exists(project_id, event_id)? {
                return Err(UmweltError::not_found(format!(
                    "Witnessed event '{event_id}' does not exist."
                )));
            }
        }
        Acquisition::Evidence { .. } | Acquisition::Authored => {}
    }
    Ok(())
}

fn record_tick<V: ValidationView>(
    view: &V,
    project_id: &str,
    id: &str,
    prepared: &PreparedCommit,
) -> Result<i64> {
    if let Some(index) = prepared
        .record_ids
        .iter()
        .position(|candidate| candidate == id)
    {
        return Ok(prepared.input.records[index]
            .common()
            .position_in(&prepared.input.event)
            .tick);
    }
    view.record_tick(project_id, id)?
        .ok_or_else(|| UmweltError::not_found(format!("Record '{id}' does not exist.")))
}

fn record_common(record: &Record) -> &RecordCommon {
    match record {
        Record::Fact { common, .. }
        | Record::Holding { common, .. }
        | Record::Awareness { common, .. }
        | Record::Disposition { common, .. } => common,
    }
}

fn require_holder<V: ValidationView>(view: &V, project_id: &str, id: &str) -> Result<()> {
    match view.entity_kind(project_id, id)? {
        Some(EntityKind::Agent | EntityKind::Group) => Ok(()),
        Some(EntityKind::Other) => Err(UmweltError::validation(format!(
            "Entity '{id}' cannot be a holder."
        ))),
        None => Err(UmweltError::not_found(format!(
            "Holder entity '{id}' does not exist."
        ))),
    }
}

fn require_entity<V: ValidationView>(
    view: &V,
    project_id: &str,
    id: &str,
    role: &str,
) -> Result<()> {
    if view.entity_kind(project_id, id)?.is_some() {
        Ok(())
    } else {
        Err(UmweltError::not_found(format!(
            "Entity '{id}' referenced as {role} does not exist."
        )))
    }
}

fn require_record<V: ValidationView>(
    view: &V,
    project_id: &str,
    id: &str,
    new_records: &BTreeSet<&str>,
) -> Result<()> {
    if new_records.contains(id) || view.record_exists(project_id, id)? {
        Ok(())
    } else {
        Err(UmweltError::not_found(format!(
            "Record '{id}' does not exist."
        )))
    }
}

fn id_conflict(kind: &str, id: &str) -> UmweltError {
    UmweltError::public(
        UmweltErrorCode::Conflict,
        format!("The {kind} id '{id}' already exists."),
    )
}
