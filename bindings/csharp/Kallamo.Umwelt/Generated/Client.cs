// SPDX-License-Identifier: AGPL-3.0-only
// Generated from the versioned JSON schemas by scripts/generate-csharp.mjs. Do not edit.
#nullable enable
using System.Collections.Generic;

namespace Kallamo.Umwelt;

public sealed partial class UmweltEngine
{
    /// <summary>Protocol operation <c>axis.define</c> (command).</summary>
    public AxisDef AxisDefine(DefineAxisInput payload) => Call<AxisDef>("axis.define", payload);

    /// <summary>Protocol operation <c>axis.list</c> (query).</summary>
    public List<AxisDef> AxisList(ProjectId payload) => Call<List<AxisDef>>("axis.list", payload);

    /// <summary>Protocol operation <c>beliefsAbout</c> (query).</summary>
    public List<BeliefAboutItem> BeliefsAbout(BeliefsAboutPayload payload) => Call<List<BeliefAboutItem>>("beliefsAbout", payload);

    /// <summary>Protocol operation <c>breaches</c> (query).</summary>
    public List<Breach> Breaches(BreachesPayload payload) => Call<List<Breach>>("breaches", payload);

    /// <summary>Protocol operation <c>canon</c> (query).</summary>
    public RecordPageResult Canon(CanonPayload payload) => Call<RecordPageResult>("canon", payload);

    /// <summary>Protocol operation <c>changes</c> (query).</summary>
    public ChangePageResult Changes(ChangesPayload payload) => Call<ChangePageResult>("changes", payload);

    /// <summary>Protocol operation <c>check.utterance</c> (query).</summary>
    public CheckUtteranceResult CheckUtterance(CheckUtteranceInput payload) => Call<CheckUtteranceResult>("check.utterance", payload);

    /// <summary>Protocol operation <c>commit</c> (command).</summary>
    public CommitOutcome Commit(CommitInput payload) => Call<CommitOutcome>("commit", payload);

    /// <summary>Protocol operation <c>commitBatch</c> (command).</summary>
    public List<CommitOutcome> CommitBatch(CommitBatchPayload payload) => Call<List<CommitOutcome>>("commitBatch", payload);

    /// <summary>Protocol operation <c>compare</c> (query).</summary>
    public CompareResult Compare(ComparePayload payload) => Call<CompareResult>("compare", payload);

    /// <summary>Protocol operation <c>context.admit</c> (query).</summary>
    public ContextAdmitResult ContextAdmit(ContextAdmitInput payload) => Call<ContextAdmitResult>("context.admit", payload);

    /// <summary>Protocol operation <c>contradictions</c> (query).</summary>
    public List<Contradiction> Contradictions(HolderAt payload) => Call<List<Contradiction>>("contradictions", payload);

    /// <summary>Protocol operation <c>disposition.setBatch</c> (command).</summary>
    public CommitOutcome DispositionSetBatch(SetDispositionsInput payload) => Call<CommitOutcome>("disposition.setBatch", payload);

    /// <summary>Protocol operation <c>embedding.defineSpace</c> (command).</summary>
    public EmbeddingSpace EmbeddingDefineSpace(DefineEmbeddingSpaceInput payload) => Call<EmbeddingSpace>("embedding.defineSpace", payload);

    /// <summary>Protocol operation <c>embedding.delete</c> (command).</summary>
    public DeleteEmbeddingsResult EmbeddingDelete(DeleteEmbeddingsInput payload) => Call<DeleteEmbeddingsResult>("embedding.delete", payload);

    /// <summary>Protocol operation <c>embedding.dropSpace</c> (command).</summary>
    public DropEmbeddingSpaceResult EmbeddingDropSpace(EmbeddingSpaceRef payload) => Call<DropEmbeddingSpaceResult>("embedding.dropSpace", payload);

    /// <summary>Protocol operation <c>embedding.listSpaces</c> (query).</summary>
    public List<EmbeddingSpace> EmbeddingListSpaces(ProjectId payload) => Call<List<EmbeddingSpace>>("embedding.listSpaces", payload);

    /// <summary>Protocol operation <c>embedding.pending</c> (query).</summary>
    public PendingEmbeddingsResult EmbeddingPending(PendingEmbeddingsInput payload) => Call<PendingEmbeddingsResult>("embedding.pending", payload);

    /// <summary>Protocol operation <c>embedding.put</c> (command).</summary>
    public PutEmbeddingsResult EmbeddingPut(PutEmbeddingsInput payload) => Call<PutEmbeddingsResult>("embedding.put", payload);

    /// <summary>Protocol operation <c>entity.archive</c> (command).</summary>
    public Entity EntityArchive(EntityId payload) => Call<Entity>("entity.archive", payload);

    /// <summary>Protocol operation <c>entity.asKnownBy</c> (query).</summary>
    public EntityKnownResult EntityAsKnownBy(EntityKnownPayload payload) => Call<EntityKnownResult>("entity.asKnownBy", payload);

    /// <summary>Protocol operation <c>entity.at</c> (query).</summary>
    public EntityAtResult EntityAt(EntityAtPayload payload) => Call<EntityAtResult>("entity.at", payload);

    /// <summary>Protocol operation <c>entity.create</c> (command).</summary>
    public Entity EntityCreate(CreateEntityInput payload) => Call<Entity>("entity.create", payload);

    /// <summary>Protocol operation <c>entity.ensureBatch</c> (command).</summary>
    public List<EnsureEntityResult> EntityEnsureBatch(EnsureEntitiesInput payload) => Call<List<EnsureEntityResult>>("entity.ensureBatch", payload);

    /// <summary>Protocol operation <c>entity.get</c> (query).</summary>
    public Entity EntityGet(EntityId payload) => Call<Entity>("entity.get", payload);

    /// <summary>Protocol operation <c>entity.list</c> (query).</summary>
    public EntityPageResult EntityList(PagedProject payload) => Call<EntityPageResult>("entity.list", payload);

    /// <summary>Protocol operation <c>entity.resolve</c> (query).</summary>
    public Entity EntityResolve(ResolveEntityPayload payload) => Call<Entity>("entity.resolve", payload);

    /// <summary>Protocol operation <c>entity.resolveBatch</c> (query).</summary>
    public List<ResolvedEntity> EntityResolveBatch(ResolveEntitiesPayload payload) => Call<List<ResolvedEntity>>("entity.resolveBatch", payload);

    /// <summary>Protocol operation <c>entity.update</c> (command).</summary>
    public Entity EntityUpdate(UpdateEntityInput payload) => Call<Entity>("entity.update", payload);

    /// <summary>Protocol operation <c>entityType.create</c> (command).</summary>
    public EntityType EntityTypeCreate(CreateEntityTypeInput payload) => Call<EntityType>("entityType.create", payload);

    /// <summary>Protocol operation <c>entityType.list</c> (query).</summary>
    public List<EntityType> EntityTypeList(ProjectId payload) => Call<List<EntityType>>("entityType.list", payload);

    /// <summary>Protocol operation <c>evidence.get</c> (query).</summary>
    public EvidenceSource EvidenceGet(EvidenceId payload) => Call<EvidenceSource>("evidence.get", payload);

    /// <summary>Protocol operation <c>evidence.register</c> (command).</summary>
    public EvidenceSource EvidenceRegister(EvidenceSource payload) => Call<EvidenceSource>("evidence.register", payload);

    /// <summary>Protocol operation <c>groupCoverage</c> (query).</summary>
    public GroupCoverageResult GroupCoverage(GroupCoveragePayload payload) => Call<GroupCoverageResult>("groupCoverage", payload);

    /// <summary>Protocol operation <c>holding.adjustBatch</c> (command).</summary>
    public CommitOutcome HoldingAdjustBatch(AdjustHoldingsInput payload) => Call<CommitOutcome>("holding.adjustBatch", payload);

    /// <summary>Protocol operation <c>holdings</c> (query).</summary>
    public HoldingsResult Holdings(HoldingsPayload payload) => Call<HoldingsResult>("holdings", payload);

    /// <summary>Protocol operation <c>holds</c> (query).</summary>
    public HoldsResult Holds(HoldsPayload payload) => Call<HoldsResult>("holds", payload);

    /// <summary>Protocol operation <c>perspective</c> (query).</summary>
    public PerspectiveResult Perspective(PerspectivePayload payload) => Call<PerspectiveResult>("perspective", payload);

    /// <summary>Protocol operation <c>predicate.define</c> (command).</summary>
    public PredicateDef PredicateDefine(DefinePredicateInput payload) => Call<PredicateDef>("predicate.define", payload);

    /// <summary>Protocol operation <c>predicate.list</c> (query).</summary>
    public List<PredicateDef> PredicateList(ProjectId payload) => Call<List<PredicateDef>>("predicate.list", payload);

    /// <summary>Protocol operation <c>project.create</c> (command).</summary>
    public Project ProjectCreate(CreateProjectInput payload) => Call<Project>("project.create", payload);

    /// <summary>Protocol operation <c>project.get</c> (query).</summary>
    public Project ProjectGet(ProjectId payload) => Call<Project>("project.get", payload);

    /// <summary>Protocol operation <c>project.list</c> (query).</summary>
    public List<Project> ProjectList(EmptyPayload payload) => Call<List<Project>>("project.list", payload);

    /// <summary>Protocol operation <c>proposal.apply</c> (command).</summary>
    public CommitOutcome ProposalApply(ApplyPayload payload) => Call<CommitOutcome>("proposal.apply", payload);

    /// <summary>Protocol operation <c>proposal.get</c> (query).</summary>
    public Proposal ProposalGet(ProposalId payload) => Call<Proposal>("proposal.get", payload);

    /// <summary>Protocol operation <c>proposal.list</c> (query).</summary>
    public List<Proposal> ProposalList(ProjectId payload) => Call<List<Proposal>>("proposal.list", payload);

    /// <summary>Protocol operation <c>proposal.rebase</c> (command).</summary>
    public ProposalRebaseResult ProposalRebase(ProposalId payload) => Call<ProposalRebaseResult>("proposal.rebase", payload);

    /// <summary>Protocol operation <c>proposal.reject</c> (command).</summary>
    public Proposal ProposalReject(RejectPayload payload) => Call<Proposal>("proposal.reject", payload);

    /// <summary>Protocol operation <c>proposal.review</c> (command).</summary>
    public Proposal ProposalReview(ReviewPayload payload) => Call<Proposal>("proposal.review", payload);

    /// <summary>Protocol operation <c>propose</c> (command).</summary>
    public Proposal Propose(ProposeInput payload) => Call<Proposal>("propose", payload);

    /// <summary>Protocol operation <c>record</c> (query).</summary>
    public Record Record(RecordId payload) => Call<Record>("record", payload);

    /// <summary>Protocol operation <c>search</c> (query).</summary>
    public SearchPageResult Search(SearchPayload payload) => Call<SearchPageResult>("search", payload);

    /// <summary>Protocol operation <c>shutdown</c> (command).</summary>
    public EmptyPayload Shutdown(EmptyPayload payload) => Call<EmptyPayload>("shutdown", payload);

    /// <summary>Protocol operation <c>timeline</c> (query).</summary>
    public EventPageResult Timeline(TimelinePayload payload) => Call<EventPageResult>("timeline", payload);

    /// <summary>Protocol operation <c>timeline.discard</c> (command).</summary>
    public Timeline TimelineDiscard(DiscardPayload payload) => Call<Timeline>("timeline.discard", payload);

    /// <summary>Protocol operation <c>timeline.get</c> (query).</summary>
    public Timeline TimelineGet(TimelineRef payload) => Call<Timeline>("timeline.get", payload);

    /// <summary>Protocol operation <c>timeline.list</c> (query).</summary>
    public List<Timeline> TimelineList(TimelineListPayload payload) => Call<List<Timeline>>("timeline.list", payload);

    /// <summary>Protocol operation <c>timeline.openDraft</c> (command).</summary>
    public Timeline TimelineOpenDraft(OpenDraftInput payload) => Call<Timeline>("timeline.openDraft", payload);

    /// <summary>Protocol operation <c>timeline.promote</c> (command).</summary>
    public Timeline TimelinePromote(TimelineRef payload) => Call<Timeline>("timeline.promote", payload);

    /// <summary>Protocol operation <c>timeline.purge</c> (command).</summary>
    public PurgeResult TimelinePurge(TimelineRef payload) => Call<PurgeResult>("timeline.purge", payload);

    /// <summary>Protocol operation <c>timeline.rebase</c> (command).</summary>
    public Timeline TimelineRebase(TimelineRef payload) => Call<Timeline>("timeline.rebase", payload);

    /// <summary>Protocol operation <c>trace</c> (query).</summary>
    public TraceResult Trace(RecordId payload) => Call<TraceResult>("trace", payload);

    /// <summary>Protocol operation <c>transition.batch</c> (command).</summary>
    public CommitOutcome TransitionBatch(TransitionBatchInput payload) => Call<CommitOutcome>("transition.batch", payload);

    /// <summary>Protocol operation <c>transmit</c> (command).</summary>
    public CommitOutcome Transmit(TransmitInput payload) => Call<CommitOutcome>("transmit", payload);

    /// <summary>Protocol operation <c>transmitBatch</c> (command).</summary>
    public List<CommitOutcome> TransmitBatch(TransmitBatchPayload payload) => Call<List<CommitOutcome>>("transmitBatch", payload);

    /// <summary>Protocol operation <c>whoHolds</c> (query).</summary>
    public List<WhoHoldsItem> WhoHolds(WhoHoldsPayload payload) => Call<List<WhoHoldsItem>>("whoHolds", payload);

    /// <summary>Protocol operation <c>witness</c> (command).</summary>
    public CommitOutcome Witness(WitnessInput payload) => Call<CommitOutcome>("witness", payload);
}
