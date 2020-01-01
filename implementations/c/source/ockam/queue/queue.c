//
// Created by Robin Budd on 191220.
//
#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include "queue.h"
#include "error.h"
#include "syslog.h"

typedef struct {
	uint16_t            max_size;
	uint16_t            size;
	uint16_t            head;
	u_int16_t           tail;
	pthread_mutex_t     modify_lock;
	pthread_cond_t*     p_alert;
	void*               nodes[];
} QUEUE;

OCKAM_ERR init_queue( uint16_t max_entries, pthread_cond_t* p_alert, OCKAM_QUEUE* pp_queue )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	QUEUE*              p_queue = NULL;
	size_t              size_queue = 0;

	// Validate parameters
	if(( max_entries < 1 ) || ( NULL == pp_queue )){
		log_error( OCKAM_ERR_INVALID_PARAM, "Invalid parameter in init_queue");
		goto exit_block;
	}
	*pp_queue = NULL;

	// Allocate queue memory
	size_queue = sizeof( QUEUE ) + (max_entries-1)*(sizeof(void*));
	p_queue = ( QUEUE* )malloc( size_queue );
	if( NULL == p_queue){
		log_error( OCKAM_ERR_MEM_INSUFFICIENT, "Malloc failed in init_queue");
		goto exit_block;
	}
	memset( p_queue, 0, size_queue );
	p_queue->max_size = max_entries;

	// Create the queue lock
	status = pthread_mutex_init( &p_queue->modify_lock, NULL );
	if( 0 != status ){
		log_error( OCKAM_ERR_CREATE_MUTEX, "Mutex failed in init_queue");
		goto exit_block;
	}

	// Save the alert condition, if one was given
	if( NULL != p_alert ) p_queue->p_alert = p_alert;

	// Success
	*pp_queue = p_queue;

exit_block:
	if(( OCKAM_ERR_NONE != status ) && ( NULL != p_queue )) {
		pthread_mutex_destroy( &p_queue->modify_lock );
		free( p_queue );
	}
	return status;
};

OCKAM_ERR enqueue( OCKAM_QUEUE q, void* node )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	QUEUE*              p_q = ( QUEUE* )q;
	int16_t             q_is_locked = 0;

	// Validate parameters
	if(( NULL == q ) || ( NULL == node )){
		log_error( OCKAM_ERR_INVALID_PARAM, "Invalid parameter in enqueue");
		status = OCKAM_ERR_INVALID_PARAM;
		goto exit_block;
	}

	// Lock the queue
	if( 0 != pthread_mutex_lock( &p_q->modify_lock )){
		log_error( OCKAM_ERR_LOCK_MUTEX, "lock failed ");
		status = OCKAM_ERR_LOCK_MUTEX;
		goto exit_block;
	}
	q_is_locked = 1;

	// Check for queue full
	if( p_q->size == p_q->max_size ) {
		log_error( OCKAM_ERR_QUQUE_FULL, "queue is full");
		status = OCKAM_ERR_QUQUE_FULL;
		goto exit_block;
	}

	// Add node to queue tail and bump queue size
	p_q->nodes[ p_q->tail ] = node;
	p_q->tail = ( p_q->tail + 1 ) % p_q->max_size;
	p_q->size += 1;

	// Trigger the alert condition, if we have one
	if( NULL != p_q->p_alert ) {
		pthread_cond_signal( p_q->p_alert );
	}

exit_block:
	if( q_is_locked ) pthread_mutex_unlock( &p_q->modify_lock );
	return status;
}

OCKAM_ERR dequeue( OCKAM_QUEUE q, void** pp_node )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	QUEUE*              p_q = ( QUEUE* )q;
	int16_t             q_is_locked = 0;

	// Validate parameters
	if(( NULL == q ) || ( NULL == pp_node )) {
		log_error( OCKAM_ERR_INVALID_PARAM, "invalid parameter in dequeue");
		status = OCKAM_ERR_INVALID_PARAM;
		goto exit_block;
	}

	// Lock the queue
	if( 0 != pthread_mutex_lock( &p_q->modify_lock )) {
		log_error( OCKAM_ERR_LOCK_MUTEX, "lock failed ");
		status = OCKAM_ERR_LOCK_MUTEX;
		goto exit_block;
	}
	q_is_locked = 1;

	// Check for queue empty
	if( 0 == p_q->size ) {
		log_error( OCKAM_ERR_QUEUE_EMPTY, "queue is empty");
		status = OCKAM_ERR_QUEUE_EMPTY;
		goto exit_block;
	}
	// Dequeue node and decrease size
	*pp_node = p_q->nodes[ p_q->head ];
	p_q->nodes[ p_q->head ] = NULL;
	p_q->head = ( p_q->head + 1 )%p_q->max_size;
	p_q->size -= 1;

exit_block:
	if( q_is_locked ) pthread_mutex_unlock( &p_q->modify_lock );
	return status;
}

OCKAM_ERR uninit_queue( OCKAM_QUEUE q )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	QUEUE*              p_q = ( QUEUE* )q;
	int16_t             q_is_locked = 0;
	pthread_mutex_t     lock;

	// Validate parameters
	if( NULL == q ) {
		log_error( OCKAM_ERR_INVALID_PARAM, "invalid parameter");
		goto exit_block;
	}

	// Lock the queue
	if( 0 != pthread_mutex_lock( &p_q->modify_lock )) {
		log_error( OCKAM_ERR_LOCK_MUTEX, "lock failed");
		goto exit_block;
	}
	q_is_locked = 1;
	lock = p_q->modify_lock;

	// Free up the memory
	free( p_q );

exit_block:
	if( q_is_locked ) pthread_mutex_unlock( &lock );
	return status;
}


