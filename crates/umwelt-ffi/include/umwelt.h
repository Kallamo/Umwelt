/* SPDX-License-Identifier: AGPL-3.0-only */
/* One thread at a time per handle. */
/* Status: 0 OK, 1 INVALID_ARGUMENT, 2 OPEN_FAILED, 3 POISONED, 4 PANIC. */
/* Domain errors are JSON envelopes and do not change the umwelt_call status. */
/* A failed umwelt_open returns a non-null failed handle in *out when possible; call umwelt_last_error, then umwelt_close. */

#ifndef UMWELT_H
#define UMWELT_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * `synchronous=NORMAL`: survives an application crash; a power loss may drop
 * the most recent commits. Never corrupts the file. Default of `umwelt_open`.
 */
#define UMWELT_DURABILITY_NORMAL 0

/**
 * `synchronous=FULL`: every commit is flushed before it returns.
 */
#define UMWELT_DURABILITY_FULL 1

typedef struct umwelt_engine umwelt_engine;

typedef struct umwelt_buf {
  uint8_t *ptr;
  size_t len;
} umwelt_buf;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Returns the binary ABI version, currently 1.
 */
uint32_t umwelt_abi_version(void);

/**
 * Opens or creates a UTF-8 path without requiring a trailing NUL.
 * On OPEN_FAILED, `out` receives a failed handle queryable through `umwelt_last_error`.
 *
 * # Safety
 * Pointer arguments must be valid for their declared lengths and writable where applicable.
 */
int32_t umwelt_open(const uint8_t *path,
                    size_t path_len,
                    int32_t create,
                    struct umwelt_engine **out);

/**
 * Like `umwelt_open`, with an explicit durability
 * (`UMWELT_DURABILITY_NORMAL` or `UMWELT_DURABILITY_FULL`).
 * Any other durability value returns INVALID_ARGUMENT with a failed handle.
 *
 * # Safety
 * Pointer arguments must be valid for their declared lengths and writable where applicable.
 */
int32_t umwelt_open_with_options(const uint8_t *path,
                                 size_t path_len,
                                 int32_t create,
                                 int32_t durability,
                                 struct umwelt_engine **out);

/**
 * Dispatches one JSON protocol v1 request and returns its JSON envelope.
 * The first request must be `hello`, exactly as with `umwelt serve`.
 *
 * # Safety
 * `handle` must be live, `request` valid for `request_len`, and `out` writable.
 */
int32_t umwelt_call(struct umwelt_engine *handle,
                    const uint8_t *request,
                    size_t request_len,
                    struct umwelt_buf *out);

/**
 * Copies the last nonzero-status error as JSON into `out`.
 *
 * # Safety
 * `handle` must be live and `out` must be writable.
 */
int32_t umwelt_last_error(struct umwelt_engine *handle, struct umwelt_buf *out);

/**
 * Releases a buffer allocated by this library. A null empty buffer is accepted.
 *
 * # Safety
 * A nonempty buffer must have been returned by this library and not previously freed.
 */
void umwelt_buf_free(struct umwelt_buf buffer);

/**
 * Checkpoints the WAL when possible and releases the handle. NULL is accepted.
 *
 * # Safety
 * A non-NULL handle must have been returned by `umwelt_open` and not previously closed.
 */
void umwelt_close(struct umwelt_engine *handle);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* UMWELT_H */
