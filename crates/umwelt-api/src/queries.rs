// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;
use umwelt_core::domain::*;
use umwelt_core::{
    EventCursor, EventQuery, PendingQuery, RecordCursor, RecordQuery, Result, SearchQuery,
    SemanticQuery, Store, TimelineScope, UmweltError, UmweltErrorCode, claim_key, normalize_vector,
    slot_key, validate_doc_type,
};

use crate::Engine;

/// Read queries against one timeline. A draft sees its own rows plus main's
/// rows up to its base version. `Engine`'s own query methods read main.
pub struct TimelineReader<'a, S: Store> {
    engine: &'a Engine<S>,
    timeline_id: &'a str,
}

impl<S: Store> Engine<S> {
    /// Queries that read `timeline_id` instead of main.
    pub fn in_timeline<'a>(&'a self, timeline_id: &'a str) -> TimelineReader<'a, S> {
        TimelineReader {
            engine: self,
            timeline_id,
        }
    }

    fn main_reader(&self) -> TimelineReader<'_, S> {
        self.in_timeline(MAIN_TIMELINE_ID)
    }

    pub fn holds(
        &self,
        project_id: &str,
        holder: &str,
        claim_ref: ClaimRef,
        at: At,
    ) -> Result<HoldsResult> {
        self.main_reader().holds(project_id, holder, claim_ref, at)
    }

    pub fn holdings(
        &self,
        project_id: &str,
        holder: &str,
        filter: HoldingFilter,
        at: At,
        page: Page,
    ) -> Result<HoldingsResult> {
        self.main_reader()
            .holdings(project_id, holder, filter, at, page)
    }

    pub fn who_holds(
        &self,
        project_id: &str,
        claim_ref: ClaimRef,
        at: At,
        filter: WhoHoldsFilter,
    ) -> Result<Vec<WhoHoldsItem>> {
        self.main_reader()
            .who_holds(project_id, claim_ref, at, filter)
    }

    pub fn group_coverage(
        &self,
        project_id: &str,
        group: &str,
        claim_ref: ClaimRef,
        at: At,
    ) -> Result<GroupCoverageResult> {
        self.main_reader()
            .group_coverage(project_id, group, claim_ref, at)
    }

    pub fn beliefs_about(
        &self,
        project_id: &str,
        holder: &str,
        other: &str,
        at: At,
    ) -> Result<Vec<BeliefAboutItem>> {
        self.main_reader()
            .beliefs_about(project_id, holder, other, at)
    }

    pub fn breaches(&self, project_id: &str, owner: Option<&str>, at: At) -> Result<Vec<Breach>> {
        self.main_reader().breaches(project_id, owner, at)
    }

    pub fn check_utterance(
        &self,
        project_id: &str,
        holder: &str,
        claims: &[Claim],
        at: At,
    ) -> Result<CheckUtteranceResult> {
        self.main_reader()
            .check_utterance(project_id, holder, claims, at)
    }

    pub fn context_admit(&self, input: &ContextAdmitInput) -> Result<ContextAdmitResult> {
        self.in_timeline(input.timeline_id.as_deref().unwrap_or(MAIN_TIMELINE_ID))
            .context_admit(input)
    }

    pub fn entity_at(&self, project_id: &str, entity_id: &str, at: At) -> Result<EntityAtResult> {
        self.main_reader().entity_at(project_id, entity_id, at)
    }

    pub fn entity_known_by(
        &self,
        project_id: &str,
        entity_id: &str,
        holder: &str,
        at: At,
    ) -> Result<EntityKnownResult> {
        self.main_reader()
            .entity_known_by(project_id, entity_id, holder, at)
    }

    pub fn canon(
        &self,
        project_id: &str,
        filter: CanonFilter,
        at: At,
        page: Page,
    ) -> Result<PageResult<Record>> {
        self.main_reader().canon(project_id, filter, at, page)
    }

    pub fn perspective(
        &self,
        project_id: &str,
        holder: &str,
        at: At,
        page: Page,
    ) -> Result<PerspectiveResult> {
        self.main_reader().perspective(project_id, holder, at, page)
    }

    pub fn compare(&self, project_id: &str, a: &str, b: &str, at: At) -> Result<CompareResult> {
        self.main_reader().compare(project_id, a, b, at)
    }

    /// `compare` with `b` read at its own position (`at_b`), possibly on
    /// another axis.
    pub fn compare_across(
        &self,
        project_id: &str,
        a: &str,
        b: &str,
        at: At,
        at_b: At,
    ) -> Result<CompareResult> {
        self.main_reader()
            .compare_across(project_id, a, b, at, at_b)
    }

    pub fn contradictions(
        &self,
        project_id: &str,
        holder: &str,
        at: At,
    ) -> Result<Vec<Contradiction>> {
        self.main_reader().contradictions(project_id, holder, at)
    }

    pub fn trace(&self, project_id: &str, record_id: &str) -> Result<TraceResult> {
        self.main_reader().trace(project_id, record_id)
    }

    pub fn timeline(&self, project_id: &str, at: At, page: Page) -> Result<PageResult<Event>> {
        self.main_reader().timeline(project_id, at, page)
    }

    pub fn search(
        &self,
        project_id: &str,
        text: &str,
        filter: SearchFilter,
        at: At,
        page: Page,
    ) -> Result<PageResult<SearchItem>> {
        self.main_reader()
            .search(project_id, text, filter, at, page)
    }

    /// Search by text, by meaning (`semantic`) or both (hybrid). See
    /// `TimelineReader::search_with`.
    pub fn search_with(
        &self,
        project_id: &str,
        text: Option<&str>,
        semantic: Option<SearchSemantic>,
        filter: SearchFilter,
        at: At,
        page: Page,
    ) -> Result<PageResult<SearchItem>> {
        self.main_reader()
            .search_with(project_id, text, semantic, filter, at, page)
    }

    /// Searchable documents whose vector is missing or stale, with the text
    /// to embed.
    pub fn pending_embeddings(
        &self,
        input: &PendingEmbeddingsInput,
    ) -> Result<PendingEmbeddingsResult> {
        self.in_timeline(input.timeline_id.as_deref().unwrap_or(MAIN_TIMELINE_ID))
            .pending_embeddings(input)
    }

    pub fn entity(&self, project_id: &str, entity_id: &str) -> Result<Entity> {
        self.store.entity(project_id, entity_id)
    }

    pub fn entities(&self, project_id: &str, page: Page) -> Result<PageResult<Entity>> {
        let limit = page_limit(&page)?;
        let cursor = page.cursor.as_deref().map(decode_text_cursor).transpose()?;
        let result = self.store.entities(project_id, limit, cursor.as_deref())?;
        Ok(PageResult {
            items: result.entities,
            next_cursor: result.next_cursor.as_deref().map(encode_text_cursor),
        })
    }

    pub fn predicates(&self, project_id: &str) -> Result<Vec<PredicateDef>> {
        self.store.predicates(project_id)
    }

    /// The change feed of every timeline, in commit order.
    pub fn changes(
        &self,
        project_id: &str,
        since_seq: u64,
        limit: u32,
    ) -> Result<PageResult<ChangeItem>> {
        self.changes_in(project_id, since_seq, limit, None)
    }

    /// The change feed, optionally narrowed to one timeline. The cursor is
    /// global either way.
    pub fn changes_in(
        &self,
        project_id: &str,
        since_seq: u64,
        limit: u32,
        timeline_id: Option<&str>,
    ) -> Result<PageResult<ChangeItem>> {
        let limit = validate_limit(limit)?;
        let result = self
            .store
            .changes(project_id, since_seq, limit, timeline_id)?;
        Ok(PageResult {
            items: result.changes,
            next_cursor: result
                .next_seq
                .map(|seq| encode_text_cursor(&seq.to_string())),
        })
    }
}

impl<S: Store> TimelineReader<'_, S> {
    pub fn holds(
        &self,
        project_id: &str,
        holder: &str,
        claim_ref: ClaimRef,
        at: At,
    ) -> Result<HoldsResult> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let key = resolve_claim_key(claim_ref);
        match self
            .engine
            .store
            .latest_holding(project_id, &scope, holder, &key, tick, version)?
        {
            Some(summary) => Ok(HoldsResult {
                holds: true,
                attitude: Some(summary.attitude),
                polarity: Some(summary.polarity),
                confidence: Some(summary.confidence),
                record_id: Some(summary.record_id),
            }),
            None => Ok(HoldsResult {
                holds: false,
                attitude: None,
                polarity: None,
                confidence: None,
                record_id: None,
            }),
        }
    }

    pub fn holdings(
        &self,
        project_id: &str,
        holder: &str,
        filter: HoldingFilter,
        at: At,
        page: Page,
    ) -> Result<HoldingsResult> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let limit = page_limit(&page)?;
        let cursor = page
            .cursor
            .as_deref()
            .map(decode_record_cursor)
            .transpose()?;
        let mut query = record_query(project_id, &scope, tick, version, limit, cursor)
            .holder(holder)
            .kind("holding")
            .finish();
        query.predicate = filter.predicate.clone();
        query.about_entity_id = filter.about.clone();
        query.minimum_confidence = filter.min_confidence;
        query.include_withheld = filter.include_withheld;
        query.attitudes = filter.attitude.clone();
        let result = self.engine.store.query_records(&query)?;
        let withheld_count = self.engine.store.count_withheld(&query)?;
        let items = result
            .records
            .into_iter()
            .filter(|record| matches_holding_attitudes(record, &filter.attitude))
            .filter_map(holding_item)
            .collect::<Vec<_>>();
        Ok(HoldingsResult {
            items,
            next_cursor: result.next_cursor.as_ref().map(encode_record_cursor),
            withheld_count,
        })
    }

    pub fn who_holds(
        &self,
        project_id: &str,
        claim_ref: ClaimRef,
        at: At,
        filter: WhoHoldsFilter,
    ) -> Result<Vec<WhoHoldsItem>> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let key = resolve_claim_key(claim_ref);
        let members = filter
            .member_of
            .as_deref()
            .map(|group| self.members(project_id, &scope, group, tick, version))
            .transpose()?;
        let records = self.all_claim_records(
            record_query(project_id, &scope, tick, version, 1000, None)
                .claim_key(&key)
                .kind("holding")
                .finish(),
        )?;
        Ok(records
            .into_iter()
            .filter_map(|record| match record {
                Record::Holding {
                    common,
                    holder,
                    attitude,
                    claim,
                    confidence,
                    ..
                } if (filter.attitude.is_empty() || filter.attitude.contains(&attitude))
                    && members
                        .as_ref()
                        .is_none_or(|members| members.contains(&holder)) =>
                {
                    if filter.polarity.is_none_or(|value| value == claim.polarity) {
                        Some(WhoHoldsItem {
                            holder,
                            record_id: common.id,
                            attitude,
                            polarity: claim.polarity,
                            confidence,
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect())
    }

    /// Which members of `group` hold a claim, which hold nothing about it,
    /// and whether the group itself holds it. Membership is read from active
    /// `umwelt:member-of` Facts at the read position.
    pub fn group_coverage(
        &self,
        project_id: &str,
        group: &str,
        claim_ref: ClaimRef,
        at: At,
    ) -> Result<GroupCoverageResult> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let members = self.members(project_id, &scope, group, tick, version)?;
        let key = resolve_claim_key(claim_ref);
        let pinned = At::pinned(scope.axis.clone(), tick, version);
        let holders = self.who_holds(
            project_id,
            ClaimRef::ClaimKey {
                claim_key: key.clone(),
            },
            pinned,
            WhoHoldsFilter::default(),
        )?;
        let group_holding = holders.iter().find(|item| item.holder == group).cloned();
        let holding = holders
            .into_iter()
            .filter(|item| members.contains(&item.holder))
            .collect::<Vec<_>>();
        let missing = members
            .iter()
            .filter(|member| !holding.iter().any(|item| &item.holder == *member))
            .cloned()
            .collect();
        Ok(GroupCoverageResult {
            group: group.to_owned(),
            members: members.into_iter().collect(),
            group_holding,
            holding,
            missing,
        })
    }

    /// What `holder` thinks `other` knows, compared with what `other` holds.
    pub fn beliefs_about(
        &self,
        project_id: &str,
        holder: &str,
        other: &str,
        at: At,
    ) -> Result<Vec<BeliefAboutItem>> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let beliefs = self.naming_records(
            project_id,
            &scope,
            (tick, version),
            Some(holder),
            other,
            HOLDS_PREDICATE_ID,
        )?;
        let mut items = Vec::with_capacity(beliefs.len());
        for record in beliefs {
            let Record::Holding {
                common,
                attitude,
                claim,
                confidence,
                ..
            } = record
            else {
                continue;
            };
            let Some(key) = held_claim_key(&claim) else {
                continue;
            };
            let actual = self
                .engine
                .store
                .latest_holding(project_id, &scope, other, &key, tick, version)?;
            items.push(BeliefAboutItem {
                record_id: common.id,
                claim_key: key,
                polarity: claim.polarity,
                attitude,
                confidence,
                actually_holds: actual.is_some(),
                actual_record_id: actual.map(|summary| summary.record_id),
            });
        }
        Ok(items)
    }

    /// Every time a secret was told to someone outside its circle, up to the
    /// read position, optionally only for the secrets of `owner`. Telling is
    /// never blocked; this is where it shows up.
    pub fn breaches(&self, project_id: &str, owner: Option<&str>, at: At) -> Result<Vec<Breach>> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let mut breaches = Vec::new();
        for secret in self
            .engine
            .store
            .secret_holdings(project_id, &scope, owner, tick, version)?
        {
            let Record::Holding {
                common,
                holder: secret_owner,
                secrecy: Some(secrecy),
                ..
            } = secret
            else {
                continue;
            };
            for told in self
                .engine
                .store
                .holdings_told_from(project_id, &scope, &common.id, tick, version)?
            {
                let Record::Holding {
                    common: told_common,
                    holder: recipient,
                    acquisition:
                        Acquisition::ToldBy {
                            source_holder_id, ..
                        },
                    ..
                } = told
                else {
                    continue;
                };
                if recipient == secret_owner || secrecy.shared_with.contains(&recipient) {
                    continue;
                }
                breaches.push(Breach {
                    secret_record_id: common.id.clone(),
                    owner: secret_owner.clone(),
                    told_by: source_holder_id,
                    recipient,
                    record_id: told_common.id,
                    position: told_common.position,
                    event_id: told_common.event_id,
                });
            }
        }
        breaches.sort_by(|left, right| {
            (left.position.tick, &left.record_id).cmp(&(right.position.tick, &right.record_id))
        });
        Ok(breaches)
    }

    /// Checks claims a holder says or thinks against what the holder holds,
    /// and separately against canon. Each verdict names the record behind it.
    pub fn check_utterance(
        &self,
        project_id: &str,
        holder: &str,
        claims: &[Claim],
        at: At,
    ) -> Result<CheckUtteranceResult> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let functional = self
            .engine
            .store
            .predicates(project_id)?
            .into_iter()
            .filter(|predicate| predicate.cardinality == Cardinality::One)
            .map(|predicate| predicate.id)
            .collect::<BTreeSet<_>>();
        let mut items = Vec::with_capacity(claims.len());
        for claim in claims {
            let key = claim_key(claim);
            let slot = functional
                .contains(&claim.predicate)
                .then(|| slot_key(claim));
            let mut check = UtteranceCheck {
                claim_key: key.clone(),
                verdict: UtteranceVerdict::Unknown,
                attitude: None,
                confidence: None,
                record_id: None,
                canon: CanonStatus::NoCanon,
                canon_record_id: None,
            };
            let held = self.holdings_with_key(project_id, &scope, holder, &key, tick, version)?;
            let available = latest(held.iter().filter(|record| is_available(record)));
            if let Some(Record::Holding {
                common,
                attitude,
                claim: held_claim,
                confidence,
                ..
            }) = available
            {
                check.verdict = if held_claim.polarity == claim.polarity {
                    UtteranceVerdict::Holds
                } else {
                    UtteranceVerdict::HoldsOpposite
                };
                check.attitude = Some(attitude.clone());
                check.confidence = Some(*confidence);
                check.record_id = Some(common.id.clone());
            } else if let Some(withheld) = latest(held.iter()) {
                check.verdict = UtteranceVerdict::Withheld;
                check.record_id = Some(record_common(withheld).id.clone());
            } else if let Some(other) = match &slot {
                Some(slot) => latest(
                    self.slot_records(project_id, &scope, slot, "holding", tick, version)?
                        .iter()
                        .filter(|record| {
                            is_available(record)
                                && matches!(record, Record::Holding { holder: owner, .. } if owner == holder)
                                && record_claim(record).is_some_and(|value| value.object != claim.object)
                        }),
                )
                .cloned(),
                None => None,
            } {
                check.verdict = UtteranceVerdict::HoldsOtherValue;
                check.record_id = Some(record_common(&other).id.clone());
            } else if let Some(later) = latest(
                self.holdings_with_key(project_id, &scope, holder, &key, i64::MAX, version)?
                    .iter(),
            ) {
                check.verdict = UtteranceVerdict::Future;
                check.record_id = Some(record_common(later).id.clone());
            }

            let facts = self.all_claim_records(
                record_query(project_id, &scope, tick, version, 1000, None)
                    .claim_key(&key)
                    .kind("fact")
                    .finish(),
            )?;
            if let Some(fact) = latest(facts.iter()) {
                let agrees =
                    record_claim(fact).is_some_and(|value| value.polarity == claim.polarity);
                check.canon = if agrees {
                    CanonStatus::Agrees
                } else {
                    CanonStatus::Contradicts
                };
                check.canon_record_id = Some(record_common(fact).id.clone());
            } else if let Some(slot) = &slot {
                let slot_facts =
                    self.slot_records(project_id, &scope, slot, "fact", tick, version)?;
                if let Some(fact) = latest(slot_facts.iter().filter(|record| {
                    record_claim(record)
                        .is_some_and(|value| value.polarity && value.object != claim.object)
                })) {
                    check.canon = CanonStatus::Contradicts;
                    check.canon_record_id = Some(record_common(fact).id.clone());
                }
            }
            items.push(check);
        }
        let consistent = items
            .iter()
            .all(|item| item.verdict == UtteranceVerdict::Holds);
        Ok(CheckUtteranceResult { items, consistent })
    }

    /// The holder's active Holdings of one claim key, withheld ones included.
    fn holdings_with_key(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder: &str,
        key: &str,
        tick: i64,
        version: Version,
    ) -> Result<Vec<Record>> {
        let mut query = record_query(project_id, scope, tick, version, 1000, None)
            .holder(holder)
            .claim_key(key)
            .kind("holding")
            .finish();
        query.include_withheld = true;
        self.all_claim_records(query)
    }

    /// Active records of `kind` in one functional slot (any holder).
    fn slot_records(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        slot: &str,
        kind: &str,
        tick: i64,
        version: Version,
    ) -> Result<Vec<Record>> {
        self.all_claim_records(
            record_query(project_id, scope, tick, version, 1000, None)
                .slot_key(slot)
                .kind(kind)
                .finish(),
        )
    }

    /// Decides which candidates may enter a context seen from one perspective.
    /// Checks run in a fixed order (project, time, status, perspective,
    /// accessibility, classification, duplication, budget) and every refusal
    /// names the first check that failed.
    pub fn context_admit(&self, input: &ContextAdmitInput) -> Result<ContextAdmitResult> {
        if input.include_withheld && !input.authorized_narrator {
            return Err(UmweltError::validation(
                "includeWithheld requires authorizedNarrator.",
            ));
        }
        let project_id = input.project_id.as_str();
        let (scope, tick, version) = self.resolve_at(project_id, input.at.clone())?;
        let mut rejected = Vec::new();
        let mut survivors: Vec<(usize, &ContextCandidate)> = Vec::new();
        let mut seen = BTreeSet::new();
        for (index, candidate) in input.candidates.iter().enumerate() {
            let reason = match (&candidate.record_id, &candidate.event_id) {
                (Some(record_id), None) => {
                    self.record_rejection(input, &scope, tick, version, record_id)?
                }
                (None, Some(event_id)) => {
                    self.event_rejection(input, &scope, tick, version, event_id)?
                }
                _ => Some(ContextRejection::InvalidCandidate),
            };
            let reason = reason.or_else(|| {
                candidate
                    .classification
                    .as_ref()
                    .filter(|value| input.exclude_classifications.contains(value))
                    .map(|_| ContextRejection::ExcludedClassification)
            });
            let reason = reason.or_else(|| {
                let target = (candidate.record_id.clone(), candidate.event_id.clone());
                (!seen.insert(target)).then_some(ContextRejection::Duplicate)
            });
            match reason {
                Some(reason) => rejected.push((index, rejection(candidate, reason))),
                None => survivors.push((index, candidate)),
            }
        }
        survivors.sort_by(|(left_index, left), (right_index, right)| {
            right
                .priority
                .cmp(&left.priority)
                .then(left_index.cmp(right_index))
        });
        let mut admitted = Vec::new();
        let mut used_budget = 0_u64;
        for (index, candidate) in survivors {
            let cost = u64::from(candidate.cost);
            if input
                .budget
                .is_some_and(|budget| used_budget + cost > budget)
            {
                rejected.push((index, rejection(candidate, ContextRejection::OverBudget)));
                continue;
            }
            used_budget += cost;
            admitted.push(ContextAdmitted {
                id: candidate.id.clone(),
                record_id: candidate.record_id.clone(),
                event_id: candidate.event_id.clone(),
                cost: candidate.cost,
                priority: candidate.priority,
            });
        }
        rejected.sort_by_key(|(index, _)| *index);
        Ok(ContextAdmitResult {
            admitted,
            rejected: rejected.into_iter().map(|(_, value)| value).collect(),
            used_budget,
        })
    }

    fn record_rejection(
        &self,
        input: &ContextAdmitInput,
        scope: &TimelineScope,
        tick: i64,
        version: Version,
        record_id: &str,
    ) -> Result<Option<ContextRejection>> {
        let store = &self.engine.store;
        if found(store.record(&input.project_id, scope, record_id))?.is_none() {
            return Ok(Some(ContextRejection::NotInProject));
        }
        let Some(state) =
            found(store.record_state(&input.project_id, scope, record_id, tick, version))?
        else {
            return Ok(Some(match &scope.axis {
                Some(axis) => {
                    let record = store.record(&input.project_id, scope, record_id)?;
                    let event =
                        store.event(&input.project_id, scope, &record_common(&record).event_id)?;
                    if event.on.contains_key(axis) {
                        ContextRejection::Future
                    } else {
                        ContextRejection::NotOnAxis
                    }
                }
                None => ContextRejection::Future,
            }));
        };
        if state.transition.is_some() {
            return Ok(Some(ContextRejection::Inactive));
        }
        if let Some(holder) = &input.holder {
            let owner = match &state.record {
                Record::Fact { .. } => None,
                Record::Holding { holder, .. }
                | Record::Awareness { holder, .. }
                | Record::Disposition { holder, .. } => Some(holder),
            };
            if owner != Some(holder) {
                return Ok(Some(ContextRejection::NotHeld));
            }
        }
        if let Record::Holding { accessibility, .. } = &state.record {
            if *accessibility != Accessibility::Available && !input.include_withheld {
                return Ok(Some(ContextRejection::Withheld));
            }
        }
        Ok(None)
    }

    fn event_rejection(
        &self,
        input: &ContextAdmitInput,
        scope: &TimelineScope,
        tick: i64,
        version: Version,
        event_id: &str,
    ) -> Result<Option<ContextRejection>> {
        let store = &self.engine.store;
        let Some(event) = found(store.event(&input.project_id, scope, event_id))? else {
            return Ok(Some(ContextRejection::NotInProject));
        };
        let event_tick = match &scope.axis {
            Some(axis) => match event.on.get(axis) {
                Some(position) => *position,
                None => return Ok(Some(ContextRejection::NotOnAxis)),
            },
            None => event.position.tick,
        };
        if event_tick > tick || event.recorded_at > version {
            return Ok(Some(ContextRejection::Future));
        }
        // Being present grants nothing: a holder sees an event only through a
        // Holding witnessed there.
        if let Some(holder) = &input.holder {
            if !store.witnessed_by(&input.project_id, scope, holder, event_id, tick, version)? {
                return Ok(Some(ContextRejection::NotHeld));
            }
        }
        Ok(None)
    }

    /// The entity as canon describes it at the read position.
    pub fn entity_at(&self, project_id: &str, entity_id: &str, at: At) -> Result<EntityAtResult> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let entity = self.engine.store.entity(project_id, entity_id)?;
        let window = (tick, version);
        let names = self.naming_records(
            project_id,
            &scope,
            window,
            None,
            entity_id,
            NAME_PREDICATE_ID,
        )?;
        let titles = self.naming_records(
            project_id,
            &scope,
            window,
            None,
            entity_id,
            TITLE_PREDICATE_ID,
        )?;
        let aliases = self.naming_records(
            project_id,
            &scope,
            window,
            None,
            entity_id,
            ALIAS_PREDICATE_ID,
        )?;
        let naming = Naming::from_records(&names, &titles, &aliases);
        let mut alias_list = entity.aliases.clone();
        for alias in naming.aliases {
            if !alias_list.contains(&alias) {
                alias_list.push(alias);
            }
        }
        Ok(EntityAtResult {
            name: naming.name.unwrap_or_else(|| entity.name.clone()),
            title: naming.title,
            aliases: alias_list,
            record_ids: naming.record_ids,
            entity,
        })
    }

    /// The entity as `holder` knows it: names, title and aliases from the
    /// holder's own active, available Holdings only.
    pub fn entity_known_by(
        &self,
        project_id: &str,
        entity_id: &str,
        holder: &str,
        at: At,
    ) -> Result<EntityKnownResult> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        self.engine.store.entity(project_id, entity_id)?;
        let window = (tick, version);
        let names = self.naming_records(
            project_id,
            &scope,
            window,
            Some(holder),
            entity_id,
            NAME_PREDICATE_ID,
        )?;
        let titles = self.naming_records(
            project_id,
            &scope,
            window,
            Some(holder),
            entity_id,
            TITLE_PREDICATE_ID,
        )?;
        let aliases = self.naming_records(
            project_id,
            &scope,
            window,
            Some(holder),
            entity_id,
            ALIAS_PREDICATE_ID,
        )?;
        let naming = Naming::from_records(&names, &titles, &aliases);
        let aware = self
            .engine
            .store
            .perspective_records(project_id, &scope, holder, tick, version)?
            .awareness
            .iter()
            .any(
                |record| matches!(record, Record::Awareness { target, .. } if target == entity_id),
            );
        Ok(EntityKnownResult {
            entity_id: entity_id.to_owned(),
            holder: holder.to_owned(),
            aware,
            name: naming.name,
            title: naming.title,
            aliases: naming.aliases,
            record_ids: naming.record_ids,
        })
    }

    pub fn canon(
        &self,
        project_id: &str,
        filter: CanonFilter,
        at: At,
        page: Page,
    ) -> Result<PageResult<Record>> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let limit = page_limit(&page)?;
        let cursor = page
            .cursor
            .as_deref()
            .map(decode_record_cursor)
            .transpose()?;
        let mut query = record_query(project_id, &scope, tick, version, limit, cursor)
            .kind("fact")
            .finish();
        query.subject_id = filter.subject;
        query.predicate = filter.predicate;
        query.object_entity_id = filter.object;
        let result = self.engine.store.query_records(&query)?;
        Ok(PageResult {
            items: result.records,
            next_cursor: result.next_cursor.as_ref().map(encode_record_cursor),
        })
    }

    pub fn perspective(
        &self,
        project_id: &str,
        holder: &str,
        at: At,
        page: Page,
    ) -> Result<PerspectiveResult> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let holder_entity = self.engine.store.entity(project_id, holder)?;
        let perspective_records = self
            .engine
            .store
            .perspective_records(project_id, &scope, holder, tick, version)?;
        let holdings = self.holdings(
            project_id,
            holder,
            HoldingFilter::default(),
            At::pinned(scope.axis.clone(), tick, version),
            page,
        )?;
        let memberships = self.all_claim_records(
            record_query(project_id, &scope, tick, version, 1000, None)
                .subject(holder)
                .predicate(MEMBER_OF_PREDICATE_ID)
                .kind("fact")
                .finish(),
        )?;
        Ok(PerspectiveResult {
            holder: holder_entity,
            awareness: perspective_records.awareness,
            holdings: holdings.items,
            dispositions: perspective_records.dispositions,
            memberships,
            withheld_count: holdings.withheld_count,
        })
    }

    pub fn compare(&self, project_id: &str, a: &str, b: &str, at: At) -> Result<CompareResult> {
        self.compare_across(project_id, a, b, at.clone(), at)
    }

    /// Compares what `a` holds at `at` with what `b` holds at `at_b`. The two
    /// positions may be on different axes: what the reader knows in chapter
    /// 12 against what a character knows at story tick 10.
    pub fn compare_across(
        &self,
        project_id: &str,
        a: &str,
        b: &str,
        at: At,
        at_b: At,
    ) -> Result<CompareResult> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let (scope_b, tick_b, version_b) = self.resolve_at(project_id, at_b)?;
        let a_records = self.active_holdings(project_id, &scope, a, tick, version)?;
        let b_records = self.active_holdings(project_id, &scope_b, b, tick_b, version_b)?;
        let a_map = latest_by_claim_key(a_records);
        let b_map = latest_by_claim_key(b_records);
        let keys = a_map
            .keys()
            .chain(b_map.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut result = CompareResult {
            shared: Vec::new(),
            conflicting: Vec::new(),
            only_a: Vec::new(),
            only_b: Vec::new(),
            counts: CompareCounts {
                shared: 0,
                conflicting: 0,
                only_a: 0,
                only_b: 0,
            },
        };
        for key in keys {
            let left = a_map.get(&key).cloned();
            let right = b_map.get(&key).cloned();
            let entry = CompareEntry {
                claim_key: key,
                a: left.clone(),
                b: right.clone(),
            };
            match (
                left.as_ref().and_then(record_claim),
                right.as_ref().and_then(record_claim),
            ) {
                (Some(left), Some(right)) if left.polarity != right.polarity => {
                    result.conflicting.push(entry)
                }
                (Some(_), Some(_)) => result.shared.push(entry),
                (Some(_), None) => result.only_a.push(entry),
                (None, Some(_)) => result.only_b.push(entry),
                (None, None) => {}
            }
        }
        result.counts = CompareCounts {
            shared: result.shared.len() as u64,
            conflicting: result.conflicting.len() as u64,
            only_a: result.only_a.len() as u64,
            only_b: result.only_b.len() as u64,
        };
        Ok(result)
    }

    pub fn contradictions(
        &self,
        project_id: &str,
        holder: &str,
        at: At,
    ) -> Result<Vec<Contradiction>> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let holdings = self.active_holdings(project_id, &scope, holder, tick, version)?;
        let functional = self
            .engine
            .store
            .predicates(project_id)?
            .into_iter()
            .filter(|predicate| predicate.cardinality == Cardinality::One)
            .map(|predicate| predicate.id)
            .collect::<BTreeSet<_>>();
        let mut contradictions = Vec::new();
        let mut holdings_by_claim: BTreeMap<String, Vec<&Record>> = BTreeMap::new();
        let mut holdings_by_slot: BTreeMap<String, Vec<&Record>> = BTreeMap::new();
        for holding in &holdings {
            if let Some(claim) = record_claim(holding) {
                holdings_by_claim
                    .entry(claim_key(claim))
                    .or_default()
                    .push(holding);
                if functional.contains(&claim.predicate) {
                    holdings_by_slot
                        .entry(slot_key(claim))
                        .or_default()
                        .push(holding);
                }
            }
        }
        for (key, grouped_holdings) in &holdings_by_claim {
            let facts = self.all_claim_records(
                record_query(project_id, &scope, tick, version, 1000, None)
                    .claim_key(key)
                    .kind("fact")
                    .finish(),
            )?;
            for holding in grouped_holdings {
                for fact in &facts {
                    if record_claim(holding).is_some_and(|left| {
                        record_claim(fact).is_some_and(|right| left.polarity != right.polarity)
                    }) {
                        contradictions.push(contradiction(
                            holding,
                            fact,
                            ContradictionKind::VsCanon,
                            ContradictionReason::OppositePolarity,
                        ));
                    }
                }
            }
            for (index, left) in grouped_holdings.iter().enumerate() {
                for right in grouped_holdings.iter().skip(index + 1) {
                    if record_claim(left).is_some_and(|left_claim| {
                        record_claim(right)
                            .is_some_and(|right_claim| left_claim.polarity != right_claim.polarity)
                    }) {
                        contradictions.push(contradiction(
                            left,
                            right,
                            ContradictionKind::VsSelf,
                            ContradictionReason::OppositePolarity,
                        ));
                    }
                }
            }
        }
        for (key, grouped_holdings) in &holdings_by_slot {
            let facts = self.all_claim_records(
                record_query(project_id, &scope, tick, version, 1000, None)
                    .slot_key(key)
                    .kind("fact")
                    .finish(),
            )?;
            for holding in grouped_holdings {
                for fact in &facts {
                    if different_claim_object(holding, fact) {
                        contradictions.push(contradiction(
                            holding,
                            fact,
                            ContradictionKind::VsCanon,
                            ContradictionReason::DifferentValueInFunctionalSlot,
                        ));
                    }
                }
            }
            for (index, left) in grouped_holdings.iter().enumerate() {
                for right in grouped_holdings.iter().skip(index + 1) {
                    if different_claim_object(left, right) {
                        contradictions.push(contradiction(
                            left,
                            right,
                            ContradictionKind::VsSelf,
                            ContradictionReason::DifferentValueInFunctionalSlot,
                        ));
                    }
                }
            }
        }
        Ok(contradictions)
    }

    pub fn trace(&self, project_id: &str, record_id: &str) -> Result<TraceResult> {
        let (scope, _, _) = self.scope(project_id)?;
        let record = self.engine.store.record(project_id, &scope, record_id)?;
        let event = self.engine.store.event(
            project_id,
            &scope,
            record_common(&record).event_id.as_str(),
        )?;
        let transitions = self
            .engine
            .store
            .transitions_for_record(project_id, &scope, record_id)?;
        let status = transitions
            .last()
            .map(|transition| transition.status.clone());
        let mut chain = Vec::new();
        let mut next = Some(record.clone());
        let mut visited = BTreeSet::new();
        let mut truncated = false;
        while let Some(current) = next {
            if chain.len() == 32 || !visited.insert(record_common(&current).id.clone()) {
                truncated = true;
                break;
            }
            let acquisition = record_acquisition(&current).cloned();
            let source = match &acquisition {
                Some(Acquisition::ToldBy {
                    source_record_id: Some(source),
                    ..
                }) => Some(source.clone()),
                _ => None,
            };
            chain.push(ChainLink {
                record: current,
                acquisition,
            });
            next = source
                .map(|source| self.engine.store.record(project_id, &scope, &source))
                .transpose()?;
        }
        Ok(TraceResult {
            record,
            status,
            event,
            transitions,
            chain,
            truncated,
        })
    }

    pub fn timeline(&self, project_id: &str, at: At, page: Page) -> Result<PageResult<Event>> {
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let limit = page_limit(&page)?;
        let cursor = page
            .cursor
            .as_deref()
            .map(decode_record_cursor)
            .transpose()?;
        let result = self.engine.store.events(&EventQuery {
            project_id: project_id.to_owned(),
            timeline_id: scope.timeline_id.clone(),
            ancestors: scope.ancestors.clone(),
            axis: scope.axis.clone(),
            tick,
            as_of_version: version,
            limit,
            cursor: cursor.map(|cursor| EventCursor {
                tick: cursor.tick,
                recorded_version: cursor.recorded_version,
                event_id: cursor.record_id,
            }),
        })?;
        Ok(PageResult {
            items: result.events,
            next_cursor: result.next_cursor.map(|cursor| {
                encode_record_cursor(&RecordCursor {
                    tick: cursor.tick,
                    recorded_version: cursor.recorded_version,
                    record_id: cursor.event_id,
                })
            }),
        })
    }

    /// One record, if it is visible in this timeline.
    pub fn record(&self, project_id: &str, record_id: &str) -> Result<Record> {
        let (scope, _, _) = self.scope(project_id)?;
        self.engine.store.record(project_id, &scope, record_id)
    }

    /// Searches indexed text. Its cursor encodes an FTS offset and may skip or repeat
    /// items when writes occur between pages; every other read cursor is stable.
    pub fn search(
        &self,
        project_id: &str,
        text: &str,
        filter: SearchFilter,
        at: At,
        page: Page,
    ) -> Result<PageResult<SearchItem>> {
        self.search_with(project_id, Some(text), None, filter, at, page)
    }

    /// Searches by text, by meaning or both. `semantic` carries a query
    /// vector from the client's model in one of the project's embedding
    /// spaces; with `text` too, the two rankings are fused (Reciprocal Rank
    /// Fusion). Visibility is the same in every mode: a document the read
    /// cannot see is never returned, however close its vector.
    pub fn search_with(
        &self,
        project_id: &str,
        text: Option<&str>,
        semantic: Option<SearchSemantic>,
        filter: SearchFilter,
        at: At,
        page: Page,
    ) -> Result<PageResult<SearchItem>> {
        let semantic = match semantic {
            Some(semantic) => {
                let space = self
                    .engine
                    .embedding_space(project_id, &semantic.space_id)?;
                Some(SemanticQuery {
                    vector: normalize_vector(&semantic.vector, space.dimensions)?,
                    space_id: semantic.space_id,
                })
            }
            None => None,
        };
        if text.is_none() && semantic.is_none() {
            return Err(UmweltError::validation(
                "A search needs text, a semantic vector, or both.",
            ));
        }
        let text = text.unwrap_or_default();
        let (scope, tick, version) = self.resolve_at(project_id, at)?;
        let limit = page_limit(&page)?;
        let offset = page
            .cursor
            .as_deref()
            .map(decode_text_cursor)
            .transpose()?
            .map(|value| value.parse::<u64>())
            .transpose()
            .map_err(|_| UmweltError::validation("Search cursor is invalid."))?
            .unwrap_or(0);
        let result = self.engine.store.search(&SearchQuery {
            project_id: project_id.to_owned(),
            timeline_id: scope.timeline_id.clone(),
            ancestors: scope.ancestors.clone(),
            axis: scope.axis.clone(),
            text: text.to_owned(),
            semantic,
            holder_id: filter.holder,
            doc_type: filter.doc_type,
            tick,
            as_of_version: version,
            limit,
            offset,
        })?;
        Ok(PageResult {
            items: result.items,
            next_cursor: result
                .next_offset
                .map(|offset| encode_text_cursor(&offset.to_string())),
        })
    }

    /// Searchable documents visible in this timeline whose vector in a space
    /// is missing or stale. The cursor is a position in the order documents
    /// were indexed; `resumeCursor` lets a later call see only newer ones.
    pub fn pending_embeddings(
        &self,
        input: &PendingEmbeddingsInput,
    ) -> Result<PendingEmbeddingsResult> {
        for doc_type in &input.doc_types {
            validate_doc_type(doc_type)?;
        }
        let limit = page_limit(&input.page)?;
        let after = match input.page.cursor.as_deref() {
            Some(cursor) => decode_text_cursor(cursor)?
                .parse::<i64>()
                .map_err(|_| UmweltError::validation("Page cursor is invalid."))?,
            None => 0,
        };
        let (scope, _, _) = self.scope(&input.project_id)?;
        let page = self.engine.store.pending_embeddings(&PendingQuery {
            project_id: input.project_id.clone(),
            timeline_id: scope.timeline_id,
            ancestors: scope.ancestors,
            space_id: input.space_id.clone(),
            doc_types: input.doc_types.clone(),
            after,
            limit,
        })?;
        Ok(PendingEmbeddingsResult {
            items: page.items,
            next_cursor: page
                .next
                .map(|position| encode_text_cursor(&position.to_string())),
            resume_cursor: encode_text_cursor(&page.resume.to_string()),
        })
    }

    /// The read scope of this timeline with its head version and highest tick.
    /// A promoted draft now lives in its parent; a purged draft has no rows
    /// left.
    fn scope(&self, project_id: &str) -> Result<(TimelineScope, Version, i64)> {
        if self.timeline_id == MAIN_TIMELINE_ID {
            let (head, max_tick) = self.engine.store.project_window(project_id)?;
            return Ok((TimelineScope::main(), head, max_tick));
        }
        let timeline = self.engine.store.timeline(project_id, self.timeline_id)?;
        if timeline.status == TimelineStatus::Promoted {
            let parent = timeline
                .parent_timeline_id
                .as_deref()
                .unwrap_or(MAIN_TIMELINE_ID);
            return Err(UmweltError::public(
                UmweltErrorCode::InvalidState,
                format!(
                    "Timeline '{}' was promoted into '{parent}'; read '{parent}' instead.",
                    timeline.id
                ),
            )
            .with_details(json!({
                "promotedInto": parent,
                "headVersion": timeline.head_version,
            })));
        }
        if timeline.purged_at.is_some() {
            return Err(UmweltError::not_found(format!(
                "Timeline '{}' was purged.",
                timeline.id
            )));
        }
        Ok((
            self.engine.store.timeline_scope(project_id, &timeline)?,
            timeline.head_version,
            timeline.max_tick,
        ))
    }

    /// The scope, tick and version of a read. On an additional axis the
    /// scope carries the axis, and a missing tick is the highest position
    /// recorded on it among the events the timeline sees.
    fn resolve_at(&self, project_id: &str, at: At) -> Result<(TimelineScope, i64, Version)> {
        let (scope, current_version, max_tick) = self.scope(project_id)?;
        let (scope, tick) = match at.axis {
            None => (scope, at.tick.unwrap_or(max_tick)),
            Some(axis) => {
                if !self
                    .engine
                    .store
                    .axes(project_id)?
                    .iter()
                    .any(|defined| defined.id == axis)
                {
                    return Err(UmweltError::validation(format!(
                        "Axis '{axis}' is not defined in this project."
                    )));
                }
                let tick = match at.tick {
                    Some(tick) => tick,
                    None => self
                        .engine
                        .store
                        .axis_max_tick(project_id, &scope, &axis)?
                        .unwrap_or(0),
                };
                (scope.on_axis(Some(axis)), tick)
            }
        };
        Position { tick }.validate()?;
        let version = at.as_of_version.unwrap_or(current_version);
        if version > current_version {
            return Err(UmweltError::validation(if scope.is_main() {
                "asOfVersion cannot be newer than the project version."
            } else {
                "asOfVersion cannot be newer than the timeline head version."
            }));
        }
        Ok((scope, tick, version))
    }

    /// Active records of `kind` with `subject` and `predicate` (and `holder`
    /// for Holdings), oldest first.
    fn naming_records(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        (tick, version): (i64, Version),
        holder: Option<&str>,
        subject: &str,
        predicate: &str,
    ) -> Result<Vec<Record>> {
        let mut builder = record_query(project_id, scope, tick, version, 1000, None)
            .subject(subject)
            .predicate(predicate);
        builder = match holder {
            Some(holder) => builder.holder(holder).kind("holding"),
            None => builder.kind("fact"),
        };
        self.all_claim_records(builder.finish())
    }

    /// Subjects of active `umwelt:member-of` Facts whose object is `group`.
    fn members(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        group: &str,
        tick: i64,
        version: Version,
    ) -> Result<BTreeSet<String>> {
        let mut query = record_query(project_id, scope, tick, version, 1000, None)
            .predicate(MEMBER_OF_PREDICATE_ID)
            .kind("fact")
            .finish();
        query.object_entity_id = Some(group.to_owned());
        Ok(self
            .all_claim_records(query)?
            .iter()
            .filter_map(record_claim)
            .filter(|claim| claim.polarity)
            .map(|claim| claim.subject.clone())
            .collect())
    }

    fn all_claim_records(&self, mut query: RecordQuery) -> Result<Vec<Record>> {
        let mut records = Vec::new();
        loop {
            let page = self.engine.store.query_records(&query)?;
            records.extend(page.records);
            match page.next_cursor {
                Some(cursor) => query.cursor = Some(cursor),
                None => break,
            }
        }
        Ok(records)
    }

    fn active_holdings(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder: &str,
        tick: i64,
        version: Version,
    ) -> Result<Vec<Record>> {
        self.all_claim_records(
            record_query(project_id, scope, tick, version, 1000, None)
                .holder(holder)
                .kind("holding")
                .finish(),
        )
    }
}

struct RecordQueryBuilder(RecordQuery);

fn record_query(
    project_id: &str,
    scope: &TimelineScope,
    tick: i64,
    version: Version,
    limit: u32,
    cursor: Option<RecordCursor>,
) -> RecordQueryBuilder {
    RecordQueryBuilder(RecordQuery {
        project_id: project_id.to_owned(),
        timeline_id: scope.timeline_id.clone(),
        ancestors: scope.ancestors.clone(),
        axis: scope.axis.clone(),
        holder_id: None,
        record_kind: None,
        claim_key: None,
        slot_key: None,
        subject_id: None,
        predicate: None,
        object_entity_id: None,
        about_entity_id: None,
        attitudes: Vec::new(),
        minimum_confidence: None,
        include_withheld: false,
        tick,
        as_of_version: version,
        limit,
        cursor,
    })
}

impl RecordQueryBuilder {
    fn holder(mut self, value: &str) -> Self {
        self.0.holder_id = Some(value.to_owned());
        self
    }

    fn kind(mut self, value: &str) -> Self {
        self.0.record_kind = Some(value.to_owned());
        self
    }

    fn claim_key(mut self, value: &str) -> Self {
        self.0.claim_key = Some(value.to_owned());
        self
    }

    fn slot_key(mut self, value: &str) -> Self {
        self.0.slot_key = Some(value.to_owned());
        self
    }

    fn subject(mut self, value: &str) -> Self {
        self.0.subject_id = Some(value.to_owned());
        self
    }

    fn predicate(mut self, value: &str) -> Self {
        self.0.predicate = Some(value.to_owned());
        self
    }

    fn finish(self) -> RecordQuery {
        self.0
    }
}

impl From<RecordQueryBuilder> for RecordQuery {
    fn from(value: RecordQueryBuilder) -> Self {
        value.0
    }
}

fn resolve_claim_key(reference: ClaimRef) -> String {
    match reference {
        ClaimRef::Claim { claim } => claim_key(&claim),
        ClaimRef::ClaimKey { claim_key } => claim_key,
    }
}

fn holding_item(record: Record) -> Option<HoldingItem> {
    let acquisition = match &record {
        Record::Holding { acquisition, .. } => acquisition.clone(),
        _ => return None,
    };
    Some(HoldingItem {
        record,
        acquisition,
        status: None,
    })
}

fn matches_holding_attitudes(record: &Record, attitudes: &[Attitude]) -> bool {
    attitudes.is_empty()
        || matches!(record, Record::Holding { attitude, .. } if attitudes.contains(attitude))
}

fn record_common(record: &Record) -> &RecordCommon {
    match record {
        Record::Fact { common, .. }
        | Record::Holding { common, .. }
        | Record::Awareness { common, .. }
        | Record::Disposition { common, .. } => common,
    }
}

fn record_claim(record: &Record) -> Option<&Claim> {
    match record {
        Record::Fact { claim, .. } | Record::Holding { claim, .. } => Some(claim),
        Record::Awareness { .. } | Record::Disposition { .. } => None,
    }
}

fn record_acquisition(record: &Record) -> Option<&Acquisition> {
    match record {
        Record::Holding { acquisition, .. } => Some(acquisition),
        _ => None,
    }
}

fn record_order(record: &Record) -> (i64, Version, String) {
    let common = record_common(record);
    (common.position.tick, common.recorded_at, common.id.clone())
}

fn latest_by_claim_key(records: Vec<Record>) -> BTreeMap<String, Record> {
    let mut result = BTreeMap::new();
    for record in records {
        if let Some(claim) = record_claim(&record) {
            let key = claim_key(claim);
            let replace = result
                .get(&key)
                .is_none_or(|current| record_order(&record) > record_order(current));
            if replace {
                result.insert(key, record);
            }
        }
    }
    result
}

fn different_claim_object(left: &Record, right: &Record) -> bool {
    match (record_claim(left), record_claim(right)) {
        (Some(left), Some(right)) => left.object != right.object,
        _ => false,
    }
}

fn contradiction(
    holding: &Record,
    other: &Record,
    kind: ContradictionKind,
    reason: ContradictionReason,
) -> Contradiction {
    Contradiction {
        holding_id: record_common(holding).id.clone(),
        kind,
        other_record_id: record_common(other).id.clone(),
        reason,
    }
}

fn page_limit(page: &Page) -> Result<u32> {
    validate_limit(if page.limit == 0 { 100 } else { page.limit })
}

fn validate_limit(limit: u32) -> Result<u32> {
    if limit == 0 || limit > 1000 {
        return Err(UmweltError::validation(
            "Page limit must be between 1 and 1000.",
        ));
    }
    Ok(limit)
}

fn encode_record_cursor(cursor: &RecordCursor) -> String {
    encode_text_cursor(&format!(
        "{}\n{}\n{}",
        cursor.tick, cursor.recorded_version, cursor.record_id
    ))
}

fn decode_record_cursor(cursor: &str) -> Result<RecordCursor> {
    let decoded = decode_text_cursor(cursor)?;
    let mut parts = decoded.splitn(3, '\n');
    let tick = parts
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| UmweltError::validation("Page cursor is invalid."))?;
    let recorded_version = parts
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| UmweltError::validation("Page cursor is invalid."))?;
    let record_id = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| UmweltError::validation("Page cursor is invalid."))?
        .to_owned();
    Ok(RecordCursor {
        tick,
        recorded_version,
        record_id,
    })
}

fn encode_text_cursor(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_text_cursor(value: &str) -> Result<String> {
    if value.len() % 2 != 0 {
        return Err(UmweltError::validation("Page cursor is invalid."));
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| UmweltError::validation("Page cursor is invalid."))?;
    String::from_utf8(bytes).map_err(|_| UmweltError::validation("Page cursor is invalid."))
}

/// Name, title and aliases read from naming records. For the functional name
/// and title the latest record wins, and a negated latest record means "no
/// longer". Aliases are every positive alias still active.
struct Naming {
    name: Option<String>,
    title: Option<String>,
    aliases: Vec<String>,
    record_ids: Vec<String>,
}

impl Naming {
    fn from_records(names: &[Record], titles: &[Record], aliases: &[Record]) -> Self {
        let mut record_ids = Vec::new();
        let mut latest = |records: &[Record]| {
            let record = records.iter().max_by_key(|record| record_order(record))?;
            let claim = record_claim(record)?;
            if !claim.polarity {
                return None;
            }
            record_ids.push(record_common(record).id.clone());
            claim_text(claim)
        };
        let name = latest(names);
        let title = latest(titles);
        let mut alias_list = Vec::new();
        for record in aliases {
            if let Some(text) = record_claim(record)
                .filter(|claim| claim.polarity)
                .and_then(claim_text)
            {
                record_ids.push(record_common(record).id.clone());
                if !alias_list.contains(&text) {
                    alias_list.push(text);
                }
            }
        }
        Self {
            name,
            title,
            aliases: alias_list,
            record_ids,
        }
    }
}

fn claim_text(claim: &Claim) -> Option<String> {
    match &claim.object {
        ClaimObject::Value {
            value: serde_json::Value::String(text),
        } => Some(text.clone()),
        _ => None,
    }
}

/// The claim key an `umwelt:holds` claim points at.
fn held_claim_key(claim: &Claim) -> Option<String> {
    match &claim.object {
        ClaimObject::Value { value } => value
            .get("claimKey")
            .and_then(|key| key.as_str())
            .map(str::to_owned),
        _ => None,
    }
}

fn latest<'a>(records: impl Iterator<Item = &'a Record>) -> Option<&'a Record> {
    records.max_by_key(|record| record_order(record))
}

fn is_available(record: &Record) -> bool {
    matches!(
        record,
        Record::Holding {
            accessibility: Accessibility::Available,
            ..
        }
    )
}

fn rejection(candidate: &ContextCandidate, reason: ContextRejection) -> ContextRejected {
    ContextRejected {
        id: candidate.id.clone(),
        record_id: candidate.record_id.clone(),
        event_id: candidate.event_id.clone(),
        reason,
    }
}

/// `NOT_FOUND` as `None`; every other error is still an error.
fn found<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.code == UmweltErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
