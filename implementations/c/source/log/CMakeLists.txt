cmake_minimum_required(VERSION 3.13)
project(ockam_log)


###########################
# Set directory locations #
###########################

set(LOG_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../include)


#################
# Build Options #
#################

# Log Build Options
option(LOG_LINUX "LOG: Linux")
option(LOG_FREERTOS "LOG: FreeRTOS")


####################
# Log Library Code #
####################

if(LOG_LINUX)
    add_definitions(-DLOG_LINUX)
    set(LOG_SRC ${LOG_SRC_DIR}/ockam_log_linux.c)
endif()

if(LOG_FREERTOS)
    add_definitions(-DLOG_FREERTOS)
    set(LOG_SRC ${LOG_SRC_DIR}/ockam_log_freertos.c)
endif()


###################
# Generate Build! #
###################

# Add the source files
add_library(ockam_log ${LOG_SRC})

# Set the include directories
target_include_directories(ockam_log PUBLIC ${OCKAM_INC_DIR})

# Set the target
set_property(TARGET ockam_log PROPERTY C_STANDARD 99)

# Add any extra libs
target_link_libraries(ockam_log)

