
# First check if OCKAM_BASE is set
if(NOT DEFINED $ENV{OCKAM_C_BASE})
    message(FATAL_ERROR "Error: OCKAM_C_BASE is not set! CMake will exit")
endif()

set(OCKAM_C_TOOLS_BASE $ENV{OCKAM_C_BASE}/tools)
