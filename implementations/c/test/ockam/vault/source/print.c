/**
 ********************************************************************************************************
 * @file    print.c
 * @brief   Print functions for Ockam Vault tests
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <stdio.h>

#include <ockam/log.h>
#include <test_vault.h>


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

char *g_log_level_str[MAX_OCKAM_LOG] =
{
    "DEBUG",
    "INFO",
    "WARN",
    "ERROR",
    "FATAL",
};

OCKAM_LOG_e g_log_level = OCKAM_LOG_INFO;                       /* Only print log statements at info or higher            */


/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */



/**
 ********************************************************************************************************
 *                                          test_vault_print()
 *
 * @brief   Print a formated test message
 *
 * @param   level       The level at which the associated message is tied to
 *
 * @param   p_module    The vault module that the message came from
 *
 * @param   test_case   The test case number associated with the message
 *
 * @param   p_msg       The message to be printed
 *
 ********************************************************************************************************
 */

void test_vault_print(OCKAM_LOG_e level, char* p_module, uint32_t test_case, char* p_msg)
{

    if(level >= g_log_level) {
        if(test_case == TEST_VAULT_NO_TEST_CASE) {
            printf("%-10s : %5s : %s\n",
                   p_module,
                   g_log_level_str[level],
                   p_msg);
        } else {
            printf("%-10s : %5s : Test Case %02d : %s\n",
                    p_module,
                    g_log_level_str[level],
                    test_case,
                    p_msg);
        }
    }
}


/**
 ********************************************************************************************************
 *                                        test_vault_print_array()
 *
 * @brief   Handy function to print out array values in hex
 *
 * @param   level       The level at which to log to
 *
 * @param   p_module    The module printing the array
 *
 * @param   p_label     Label to print before printing the array
 *
 * @param   p_array     Array pointer to print
 *
 * @param   size        Size of the array to print
 *
 ********************************************************************************************************
 */

void test_vault_print_array(OCKAM_LOG_e level, char* p_module, char* p_label, uint8_t* p_array, uint32_t size)
{
	uint32_t i;

    if(level >= g_log_level) {
        printf("%s : %5s : %s\n",
                p_module,
                g_log_level_str[level],
                p_label);

	    for(i = 1; i <= size; i++) {
            printf("%02X ", *p_array);
            p_array++;
            if(i % 8 == 0) {
                printf("\n");
            }
        }
	    printf("\n");
    }
}

