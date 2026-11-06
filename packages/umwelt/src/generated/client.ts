// SPDX-License-Identifier: AGPL-3.0-only
// Generated from the versioned JSON schemas. Do not edit.
import type * as T from './contracts.js';
import type { StdioTransport } from '../transport.js';
export class UmweltClient {
  constructor(readonly transport: StdioTransport) {}
  axisDefine(payload: T.DefineAxisInput): Promise<T.AxisDef> { return this.transport.request("axis.define", payload); }
  axisList(payload: T.ProjectId): Promise<T.AxisListResult> { return this.transport.request("axis.list", payload); }
  beliefsAbout(payload: T.BeliefsAboutPayload): Promise<T.BeliefsAboutResult> { return this.transport.request("beliefsAbout", payload); }
  breaches(payload: T.BreachesPayload): Promise<T.BreachesResult> { return this.transport.request("breaches", payload); }
  canon(payload: T.CanonPayload): Promise<T.RecordPageResult> { return this.transport.request("canon", payload); }
  changes(payload: T.ChangesPayload): Promise<T.ChangePageResult> { return this.transport.request("changes", payload); }
  checkUtterance(payload: T.CheckUtteranceInput): Promise<T.CheckUtteranceResult> { return this.transport.request("check.utterance", payload); }
  commit(payload: T.CommitInput): Promise<T.CommitOutcome> { return this.transport.request("commit", payload); }
  commitBatch(payload: T.CommitBatchPayload): Promise<T.CommitBatchResult> { return this.transport.request("commitBatch", payload); }
  compare(payload: T.ComparePayload): Promise<T.CompareResult> { return this.transport.request("compare", payload); }
  contextAdmit(payload: T.ContextAdmitInput): Promise<T.ContextAdmitResult> { return this.transport.request("context.admit", payload); }
  contradictions(payload: T.HolderAt): Promise<T.ContradictionsResult> { return this.transport.request("contradictions", payload); }
  dispositionSetBatch(payload: T.SetDispositionsInput): Promise<T.CommitOutcome> { return this.transport.request("disposition.setBatch", payload); }
  embeddingDefineSpace(payload: T.DefineEmbeddingSpaceInput): Promise<T.EmbeddingSpace> { return this.transport.request("embedding.defineSpace", payload); }
  embeddingDelete(payload: T.DeleteEmbeddingsInput): Promise<T.DeleteEmbeddingsResult> { return this.transport.request("embedding.delete", payload); }
  embeddingDropSpace(payload: T.EmbeddingSpaceRef): Promise<T.DropEmbeddingSpaceResult> { return this.transport.request("embedding.dropSpace", payload); }
  embeddingListSpaces(payload: T.ProjectId): Promise<T.EmbeddingSpaceListResult> { return this.transport.request("embedding.listSpaces", payload); }
  embeddingPending(payload: T.PendingEmbeddingsInput): Promise<T.PendingEmbeddingsResult> { return this.transport.request("embedding.pending", payload); }
  embeddingPut(payload: T.PutEmbeddingsInput): Promise<T.PutEmbeddingsResult> { return this.transport.request("embedding.put", payload); }
  entityArchive(payload: T.EntityId): Promise<T.Entity> { return this.transport.request("entity.archive", payload); }
  entityAsKnownBy(payload: T.EntityKnownPayload): Promise<T.EntityKnownResult> { return this.transport.request("entity.asKnownBy", payload); }
  entityAt(payload: T.EntityAtPayload): Promise<T.EntityAtResult> { return this.transport.request("entity.at", payload); }
  entityCreate(payload: T.CreateEntityInput): Promise<T.Entity> { return this.transport.request("entity.create", payload); }
  entityEnsureBatch(payload: T.EnsureEntitiesInput): Promise<T.EnsureEntitiesResult> { return this.transport.request("entity.ensureBatch", payload); }
  entityGet(payload: T.EntityId): Promise<T.Entity> { return this.transport.request("entity.get", payload); }
  entityList(payload: T.PagedProject): Promise<T.EntityPageResult> { return this.transport.request("entity.list", payload); }
  entityResolve(payload: T.ResolveEntityPayload): Promise<T.Entity> { return this.transport.request("entity.resolve", payload); }
  entityResolveBatch(payload: T.ResolveEntitiesPayload): Promise<T.ResolveEntitiesResult> { return this.transport.request("entity.resolveBatch", payload); }
  entityUpdate(payload: T.UpdateEntityInput): Promise<T.Entity> { return this.transport.request("entity.update", payload); }
  entityTypeCreate(payload: T.CreateEntityTypeInput): Promise<T.EntityType> { return this.transport.request("entityType.create", payload); }
  entityTypeList(payload: T.ProjectId): Promise<T.EntityTypeListResult> { return this.transport.request("entityType.list", payload); }
  evidenceGet(payload: T.EvidenceId): Promise<T.EvidenceSource> { return this.transport.request("evidence.get", payload); }
  evidenceRegister(payload: T.EvidenceSource): Promise<T.EvidenceSource> { return this.transport.request("evidence.register", payload); }
  groupCoverage(payload: T.GroupCoveragePayload): Promise<T.GroupCoverageResult> { return this.transport.request("groupCoverage", payload); }
  holdingAdjustBatch(payload: T.AdjustHoldingsInput): Promise<T.CommitOutcome> { return this.transport.request("holding.adjustBatch", payload); }
  holdings(payload: T.HoldingsPayload): Promise<T.HoldingsResult> { return this.transport.request("holdings", payload); }
  holds(payload: T.HoldsPayload): Promise<T.HoldsResult> { return this.transport.request("holds", payload); }
  perspective(payload: T.PerspectivePayload): Promise<T.PerspectiveResult> { return this.transport.request("perspective", payload); }
  predicateDefine(payload: T.DefinePredicateInput): Promise<T.PredicateDef> { return this.transport.request("predicate.define", payload); }
  predicateList(payload: T.ProjectId): Promise<T.PredicateListResult> { return this.transport.request("predicate.list", payload); }
  projectCreate(payload: T.CreateProjectInput): Promise<T.Project> { return this.transport.request("project.create", payload); }
  projectGet(payload: T.ProjectId): Promise<T.Project> { return this.transport.request("project.get", payload); }
  projectList(payload: T.EmptyPayload): Promise<T.ProjectListResult> { return this.transport.request("project.list", payload); }
  proposalApply(payload: T.ApplyPayload): Promise<T.CommitOutcome> { return this.transport.request("proposal.apply", payload); }
  proposalGet(payload: T.ProposalId): Promise<T.Proposal> { return this.transport.request("proposal.get", payload); }
  proposalList(payload: T.ProjectId): Promise<T.ProposalListResult> { return this.transport.request("proposal.list", payload); }
  proposalRebase(payload: T.ProposalId): Promise<T.ProposalRebaseResult> { return this.transport.request("proposal.rebase", payload); }
  proposalReject(payload: T.RejectPayload): Promise<T.Proposal> { return this.transport.request("proposal.reject", payload); }
  proposalReview(payload: T.ReviewPayload): Promise<T.Proposal> { return this.transport.request("proposal.review", payload); }
  propose(payload: T.ProposeInput): Promise<T.Proposal> { return this.transport.request("propose", payload); }
  record(payload: T.RecordId): Promise<T.Record> { return this.transport.request("record", payload); }
  search(payload: T.SearchPayload): Promise<T.SearchPageResult> { return this.transport.request("search", payload); }
  shutdown(payload: T.EmptyPayload): Promise<T.EmptyPayload> { return this.transport.request("shutdown", payload); }
  timeline(payload: T.TimelinePayload): Promise<T.EventPageResult> { return this.transport.request("timeline", payload); }
  timelineDiscard(payload: T.DiscardPayload): Promise<T.Timeline> { return this.transport.request("timeline.discard", payload); }
  timelineGet(payload: T.TimelineRef): Promise<T.Timeline> { return this.transport.request("timeline.get", payload); }
  timelineList(payload: T.TimelineListPayload): Promise<T.TimelineListResult> { return this.transport.request("timeline.list", payload); }
  timelineOpenDraft(payload: T.OpenDraftInput): Promise<T.Timeline> { return this.transport.request("timeline.openDraft", payload); }
  timelinePromote(payload: T.TimelineRef): Promise<T.Timeline> { return this.transport.request("timeline.promote", payload); }
  timelinePurge(payload: T.TimelineRef): Promise<T.PurgeResult> { return this.transport.request("timeline.purge", payload); }
  timelineRebase(payload: T.TimelineRef): Promise<T.Timeline> { return this.transport.request("timeline.rebase", payload); }
  trace(payload: T.RecordId): Promise<T.TraceResult> { return this.transport.request("trace", payload); }
  transitionBatch(payload: T.TransitionBatchInput): Promise<T.CommitOutcome> { return this.transport.request("transition.batch", payload); }
  transmit(payload: T.TransmitInput): Promise<T.CommitOutcome> { return this.transport.request("transmit", payload); }
  transmitBatch(payload: T.TransmitBatchPayload): Promise<T.TransmitBatchResult> { return this.transport.request("transmitBatch", payload); }
  whoHolds(payload: T.WhoHoldsPayload): Promise<T.WhoHoldsResult> { return this.transport.request("whoHolds", payload); }
  witness(payload: T.WitnessInput): Promise<T.CommitOutcome> { return this.transport.request("witness", payload); }
  close(): Promise<void> { return this.transport.close(); }
}
