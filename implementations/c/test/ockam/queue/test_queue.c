#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "queue.h"
#include "syslog.h"

int main()
{
	char                nodes[8][2] = { "1", "2", "3", "4", "5", "6", "7", "8"};
	OCKAM_QUEUE         q = NULL;
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	void*               p_node = NULL;
	int                 ret_status = -1;

	// Initialize
	status = init_queue( 5, &q );
	if( OCKAM_ERR_NONE != status ){
		log_error( status, "Failed to init queue");
		goto exit_block;
	}

	// Try to dequeue from an empty queue
	status = dequeue( q, &p_node );
	if( OCKAM_ERR_QUEUE_EMPTY != status ) {
		log_error( 0, "Dequeue on empty queue failed");
		goto exit_block;
	}

	// Add one and take it back out
	status = enqueue( q, (void*)&nodes[0][0] );
	if( OCKAM_ERR_NONE != status ) {
		log_error( 0, "Enqueue failed" );
		goto exit_block;
	}
	status = dequeue( q, &p_node );
	if( OCKAM_ERR_NONE != status ) {
		log_error( 0, "Dequeue on populated queue failed" );
		goto exit_block;
	}
	if( 0 != strcmp( (char*)p_node, &nodes[0][0]) ) {
		log_error( 0, "Dequeue returned garbage" );
	}

	// Verify queue is empty
	status = dequeue( q, &p_node );
	if( OCKAM_ERR_QUEUE_EMPTY != status ) {
		log_error( 0, "Dequeue on empty queue failed");
		goto exit_block;
	}

	// Fill up queue, then try to add when queue full
	for( int i = 0; i < 5; ++i ) {
		status = enqueue( q, &nodes[i][0] );
		if( OCKAM_ERR_NONE != status ) {
			log_error( 0, "enqueue failed while populating queue" );
			goto exit_block;
		}
	}
	status = enqueue( q, (void*)"another " );
	if( OCKAM_ERR_QUQUE_FULL != status ) {
		log_error( 0, "enqueue didn't return queue full" );
		goto exit_block;
	}

	// Empty half-way, then refill (wrap condition)
	for( int i = 0; i < 3; ++i ) {
		status = dequeue( q, &p_node );
		if( OCKAM_ERR_NONE != status ) {
			log_error( 0, "error dequeueing while emptying half-way" );
			goto exit_block;
		}
		if( p_node != &nodes[i][0] ) {
			log_error( 0, "dequeue returned wrong node" );
			goto exit_block;
		}
	}

	// Now top of the queue, and then dequeue them all
	for( int i = 5; i < 8; ++i )
	{
		status = enqueue( q, (void*)&nodes[i] );
		if( OCKAM_ERR_NONE != status ) {
			log_error( status, "error refilling queue");
			goto exit_block;
		}
	}

	// Empty out entirely
	for( int i = 3; i < 8; ++i ) {
		status = dequeue( q, &p_node );
		if( OCKAM_ERR_NONE != status ) {
			log_error( status, "error emptying queue" );
			goto exit_block;
		}
		if( p_node != &nodes[i][0]) {
			log_error( 0, "wrong node returned");
			goto exit_block;
		}
	}

	ret_status = 0;

exit_block:
	return ret_status;
}
