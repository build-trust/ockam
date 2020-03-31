/**
 ********************************************************************************************************
 * @file  error.h
 * @brief Ockam common error defines
 ********************************************************************************************************
 */

#ifndef OCKAM_ERROR_H_
#define OCKAM_ERROR_H_

/*
 ********************************************************************************************************
 *                                               INCLUDES                                               *
 ********************************************************************************************************
 */

#include <stdint.h>

/*
 ********************************************************************************************************
 *                                               DEFINES                                                *
 ********************************************************************************************************
 */

#define kOckamErrorInterfaceMask 0xFF000000
#define kOckamErrorInterfaceShift 24u

/*
 ********************************************************************************************************
 *                                              DATA TYPES                                              *
 ********************************************************************************************************
 */

typedef uint32_t OckamError;

/*
 ********************************************************************************************************
 *                                          COMMON ERROR CODES                                          *
 ********************************************************************************************************
 */

#define kOckamErrorNone 0u
#define kOckamError 1u

#define kOckamErrorInterfaceMemory (1u << kOckamErrorInterfaceShift)
#define kOckamErrorInterfaceLog (2u << kOckamErrorInterfaceShift)
#define kOckamErrorInterfaceVault (3u << kOckamErrorInterfaceShift)
#define kOckamErrorInterfaceTransport (4u << kOckamErrorInterfaceShift)
#define kOckamErrorInterfaceKeyAgreement (5u << kOckamErrorInterfaceShift)
#define kOckamErrorCodec (6u << kOckamErrorInterfaceShift)

#endif
