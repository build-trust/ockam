/**
 ********************************************************************************************************
 * @file    stdlib.c
 * @brief   Implementation of Ockam's memory functions using stdlib calls
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <stdlib.h>
#include <string.h>

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
        if(p_buf == 0) {                                        /* Ensure the buffer pointer is not null              */
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

OCKAM_ERR ockam_mem_alloc(void** p_buf, uint32_t size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(size == 0) {                                         /* Ensure the requested size is >0                    */
            ret_val = OCKAM_ERR_INVALID_SIZE;
            break;
        }

        *p_buf = malloc(size);                                  /* Attempt to malloc                                  */

        if(*p_buf == 0) {                                       /* Check if we got a buffer                           */
            ret_val = OCKAM_ERR_MEM_UNAVAIL;
            break;
        }

        memset(*p_buf, 0, size);                                /* Always zero out the memory allocated               */
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
        if(p_buf == 0) {                                        /* Ensure the buffer point is not null                */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        free(p_buf);                                            /* Free the buffer                                    */

    } while(0);

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                          ockam_mem_copy()
 *
 * @brief   Copy data from the source buffer to the target buffer
 *
 * @param   p_target[in]    Buffer address to write data to
 *
 * @param   p_source[in]    Buffer address to get data to write from
 *
 * @param   length          Amount of data to copy
 *
 * @return  OCKAM_ERR_NONE on success.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_mem_copy(void* p_target,
                         void* p_source,
                         uint32_t length)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;

    do {
        if((p_target == 0) || (p_source == 0)) {                /* Target and source MUST be valid buffers            */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        memcpy(p_target, p_source, length);

    } while(0);

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                          ockam_mem_set()
 *
 * @brief   Set the memory buffer to the specified value
 *
 * @param   p_target[in]    Buffer address to write data to
 *
 * @param   value[in]       Value to set to the memory buffer
 *
 * @param   num             The number of bytes to set to the value
 *
 * @return  OCKAM_ERR_NONE on success.
 *
 ********************************************************************************************************
 */

OCKAM_ERR ockam_mem_set(void* p_target,
                        uint8_t value,
                        uint32_t num)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(p_target == 0) {                                     /* Target MUST be a valid buffer                      */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        memset(p_target, value, num);

    } while(0);

    return ret_val;
}

