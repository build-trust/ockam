/**
 ********************************************************************************************************
 * @file        malloc.c
 * @brief   
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <stdlib.h>

#include <ockam/define.h>
#include <ockam/error.h>
#include <ockam/kal.h>
#include <ockam/memory.h>


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

OCKAM_KAL_MUTEX g_ockam_mem_mutex;


/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */



/**
 ********************************************************************************************************
 *                                          ockam_mem_init()
 *
 * @brief   Initialize the Ockam Memory functions
 *
 * @param   p_buf[in]   The buffer to use as a chunk of memory to allocate from
 * 
 * @return  OCKAM_ERR_NONE on success.
 * 
 ********************************************************************************************************
 */

OCKAM_ERR ockam_mem_init(void* p_buf)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(p_buf == OCKAM_NULL) {                               /* Ensure the buffer pointer is not null              */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        ret_val = ockam_kal_mutex_init(&g_ockam_mem_mutex);     /* Create a memory mutex                              */
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }


    } while(0);

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                          ockam_mem_alloc()
 *
 * @brief   Allocate the specified amount of memory
 *
 * @param   p_buf[out]  The pointer to place the address of the allocated memory in. 
 *
 * @param   size[in]    The number of bytes to allocate
 * 
 * @return  OCKAM_ERR_NONE on success. OCKAM_ERR_MEM_INSUFFICIENT when not enough space.
 * 
 ********************************************************************************************************
 */

OCKAM_ERR ockam_mem_alloc(void* p_buf, uint32_t size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(size == 0) {                                         /* Ensure the requested size is >0                    */
            ret_val = OCKAM_ERR_INVALID_SIZE;
            break;
        }

        p_buf = malloc(size);                                   /* Attempt to malloc                                  */

        if(p_buf == OCKAM_NULL) {                               /* Check if we got a buffer                           */
            ret_val = OCKAM_ERR_MEM_UNAVAIL;
            break;
        }
    } while(0);

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                          ockam_mem_free()
 *
 * @brief   Free the specified memory buffer
 *
 * @param   p_buf[in]   Buffer address to free
 *
 * @return  OCKAM_ERR_NONE on success. OCKAM_ERR_MEM_INVALID_PTR if not a managed buffer.
 * 
 ********************************************************************************************************
 */

OCKAM_ERR ockam_mem_free(void* p_buf)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(p_buf == OCKAM_NULL) {                               /* Ensure the buffer point is not null                */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        free(p_buf);                                            /* Free the buffer                                    */

    } while(0);

    return ret_val;
}

