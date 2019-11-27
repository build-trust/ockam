//
// Created by Robin Budd on 191220.
//

#ifndef SERVER_QUEUE_H
#define SERVER_QUEUE_H
#include <stdlib.h>
#include <pthread.h>
#include "error.h"

typedef void* OCKAM_QUEUE;

OCKAM_ERR init_queue( uint16_t max_entries, pthread_cond_t* p_alert, OCKAM_QUEUE* pp_queue );
OCKAM_ERR enqueue( OCKAM_QUEUE q, void* node );
OCKAM_ERR dequeue( OCKAM_QUEUE q, void** node );
OCKAM_ERR uninit_queue( OCKAM_QUEUE q );
OCKAM_ERR queue_size( OCKAM_QUEUE q, uint16_t* p_size );

#endif //SERVER_QUEUE_H
