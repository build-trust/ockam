cmake_minimum_required(VERSION 3.13)
project(ockam_kal)


###########################
# Set directory locations #
###########################

set(KAL_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../include)


#################
# Build Options #
#################

option(KAL_LINUX "KAL: Linux")
option(KAL_FREERTOS "KAL: FreeRTOS")


####################
# KAL Library Code #
####################

if(KAL_LINUX)
add_definitions(-DKAL_LINUX)
set(KAL_SRC ${KAL_SRC_DIR}/ockam_kal_linux.c)
endif()

if(KAL_FREERTOS)
add_definitions(-DKAL_FREERTOS)
set(KAL_SRC ${KAL_SRC_DIR}/ockam_kal_freertos.c)
endif()


###################
# Generate Build! #
###################

# Add the source files
add_library(ockam_kal ${KAL_SRC})

# Set the include directories
target_include_directories(ockam_kal PUBLIC ${OCKAM_INC_DIR})

# Set the target
set_property(TARGET ockam_kal PROPERTY C_STANDARD 99)

# Add any extra libs
target_link_libraries(ockam_kal)

