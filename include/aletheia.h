#ifndef ALETHEIA_H
#define ALETHEIA_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AletheiaSession AletheiaSession;

// Create a stateful in-memory verification session
AletheiaSession* aletheia_session_create(uint64_t initial_fuel);

// Destroy a session and reclaim memory
void aletheia_session_free(AletheiaSession* session);

// Fork a session into a child branch in < 1 microsecond (Micro-CoW)
AletheiaSession* aletheia_session_fork(AletheiaSession* session);

// Evaluate code snippet. Returns 0 on success, 1 on trapped error.
int aletheia_eval(AletheiaSession* session, const char* code, char* out_buf, int out_len);

#ifdef __cplusplus
}
#endif

#endif // ALETHEIA_H
