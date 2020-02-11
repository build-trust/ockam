cmake_minimum_required(VERSION 3.13)
project(ockam_log)


###########################
# Set directory locations #
###########################

set(LOG_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR ${OCKAM_C_BASE}/include)


#################
# Build Options #
#################

# Log Build Options
option(LOG_SYSLOG "LOG: syslog")
option(LOG_PRINTF "LOG: printf")


####################
# Log Library Code #
####################

if(LOG_SYSLOG)
    add_definitions(-DLOG_SYSLOG)
    set(LOG_SRC ${LOG_SRC_DIR}/syslog.c)
endif()

if(LOG_PRINTF)
    add_definitions(-DLOG_PRINTF)
    set(LOG_SRC ${LOG_SRC_DIR}/printf.c)
endif()


###################
# Generate Build! #
###################

# Add the source files
add_library(ockam_log STATIC ${LOG_SRC})

# Set the include directories
target_include_directories(ockam_log PUBLIC ${OCKAM_INC_DIR})

# Set the target
set_property(TARGET ockam_log PROPERTY C_STANDARD 99)

# Add any extra libs
target_link_libraries(ockam_log)

