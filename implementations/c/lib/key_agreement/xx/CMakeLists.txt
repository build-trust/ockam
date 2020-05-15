# Ockam Transport

project(key_agreement_xx C)

message(STATUS "------------------ key_agreement_xx ---------------")
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

if(${OCKAM_BUILD_TESTS})
 add_subdirectory(tests)
 message(STATUS "-----TEST posix_tcp_transport--------")
endif()
