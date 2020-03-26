# Ockam Transport

project(key_agreement_xx C)

message(STATUS "------------------ ockam_transport ---------------")
ockam_cc_library(
   NAME
     xx
   SRCS
     "xx_common.c"
     "xx_initiator.c"
     "xx_responder.c"
  HDRS
    "xx_local.h"
   PUBLIC )
