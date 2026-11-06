// SPDX-License-Identifier: AGPL-3.0-only
#nullable enable
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;

namespace Kallamo.Umwelt;

/// <summary>
/// The application's embedding model, bound to one embedding space. Umwelt
/// never runs a model: it lists the text to embed, stores the vectors and
/// searches them.
/// </summary>
public interface IEmbedder
{
    /// <summary>The space the vectors belong to (<c>embedding.defineSpace</c>).</summary>
    string SpaceId { get; }

    /// <summary>Texts per <see cref="EmbedAsync"/> call when filling pending documents.</summary>
    int BatchSize => 64;

    /// <summary>One vector per text, in order, of the space's dimensions.</summary>
    Task<IReadOnlyList<IReadOnlyList<double>>> EmbedAsync(IReadOnlyList<string> texts, CancellationToken cancellationToken = default);
}

/// <summary>What <see cref="UmweltEngine.EmbedPendingAsync"/> did.</summary>
/// <param name="Embedded">Vectors stored.</param>
/// <param name="ResumeCursor">Pass it as the cursor later to embed only documents written since.</param>
public sealed record EmbedPendingResult(ulong Embedded, string ResumeCursor);

public sealed partial class UmweltEngine
{
    /// <summary>
    /// Embeds every searchable document of a timeline that has no vector (or
    /// a stale one) in the embedder's space, in batches, until none is left.
    /// Without <paramref name="cursor"/> every document is checked again.
    /// </summary>
    public async Task<EmbedPendingResult> EmbedPendingAsync(
        IEmbedder embedder,
        string projectId,
        string? timelineId = null,
        string? cursor = null,
        CancellationToken cancellationToken = default)
    {
        ulong embedded = 0;
        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var page = EmbeddingPending(new PendingEmbeddingsInput
            {
                ProjectId = projectId,
                SpaceId = embedder.SpaceId,
                TimelineId = timelineId,
                Page = new Page { Limit = (uint)Math.Clamp(embedder.BatchSize, 1, 1000), Cursor = cursor },
            });
            if (page.Items.Count > 0)
            {
                var vectors = await embedder
                    .EmbedAsync(page.Items.Select(item => item.Text).ToList(), cancellationToken)
                    .ConfigureAwait(false);
                if (vectors.Count != page.Items.Count)
                {
                    throw new InvalidOperationException(
                        $"The embedder returned {vectors.Count} vectors for {page.Items.Count} texts.");
                }
                var stored = EmbeddingPut(new PutEmbeddingsInput
                {
                    ProjectId = projectId,
                    SpaceId = embedder.SpaceId,
                    Items = page.Items.Select((item, index) => new EmbeddingItem
                    {
                        DocType = item.DocType,
                        DocId = item.DocId,
                        TextHash = item.TextHash,
                        Vector = vectors[index].ToList(),
                    }).ToList(),
                });
                embedded += stored.Stored;
            }
            if (page.NextCursor is null)
            {
                return new EmbedPendingResult(embedded, page.ResumeCursor);
            }
            cursor = page.NextCursor;
        }
    }

    /// <summary>
    /// Searches by meaning: <paramref name="text"/> becomes the query vector.
    /// With <paramref name="hybrid"/> (the default) the words are matched too
    /// and both rankings are fused. The rest of <paramref name="payload"/>
    /// (holder, position, timeline, page) applies as in any search.
    /// </summary>
    public async Task<SearchPageResult> SemanticSearchAsync(
        IEmbedder embedder,
        SearchPayload payload,
        string text,
        bool hybrid = true,
        CancellationToken cancellationToken = default)
    {
        var vectors = await embedder.EmbedAsync(new[] { text }, cancellationToken).ConfigureAwait(false);
        if (vectors.Count != 1)
        {
            throw new InvalidOperationException("The embedder must return one vector for the query.");
        }
        payload.Text = hybrid ? text : null;
        payload.Semantic = new SearchSemantic { SpaceId = embedder.SpaceId, Vector = vectors[0].ToList() };
        return Search(payload);
    }
}
