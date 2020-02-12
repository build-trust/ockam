cmake_minimum_required(VERSION 3.13)

###########
# Project #
###########

project(optiga-trust-x)

###########################
# Set directory locations #
###########################

set(OPTIGA_DIR ${CMAKE_CURRENT_SOURCE_DIR}/optiga-trust-x)
set(OPTIGA_SRC_DIR ${OPTIGA_DIR}/optiga)
set(OPTIGA_INC_DIR ${OPTIGA_DIR}/optiga/include)
set(OPTIGA_PAL_LINUX_DIR ${OPTIGA_DIR}/pal/linux)

#################
# Build Options #
#################

# Compiler Build Options
set(CMAKE_VERBOSE_MAKEFILE TRUE)

###########################
# Set include directories #
###########################

set(OPTIGA_INC ${OPTIGA_INC} ${OPTIGA_INC_DIR})
set(OPTIGA_INC ${OPTIGA_INC} ${OPTIGA_PAL_LINUX_DIR})

####################
# Set source files #
####################

set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/cmd/CommandLib.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/util/optiga_util.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/crypt/optiga_crypt.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/comms/ifx_i2c/ifx_i2c_transport_layer.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/comms/ifx_i2c/ifx_i2c.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/comms/ifx_i2c/ifx_i2c_config.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/comms/ifx_i2c/ifx_i2c_physical_layer.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/comms/ifx_i2c/ifx_i2c_data_link_layer.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/comms/optiga_comms.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/common/Util.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_SRC_DIR}/common/Logger.c)

set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_PAL_LINUX_DIR}/pal.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_PAL_LINUX_DIR}/pal_gpio.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_PAL_LINUX_DIR}/pal_i2c.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_PAL_LINUX_DIR}/pal_os_event.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_PAL_LINUX_DIR}/pal_os_lock.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_PAL_LINUX_DIR}/pal_os_timer.c)
set(OPTIGA_SRC ${OPTIGA_SRC} ${OPTIGA_PAL_LINUX_DIR}/target/rpi3/pal_ifx_i2c_config.c)


#########################################
# Configure link libraries & executable #
#########################################

add_library(optiga-trust-x STATIC ${OPTIGA_SRC})

target_include_directories(optiga-trust-x PUBLIC ${OPTIGA_INC})

target_link_libraries(optiga-trust-x rt)

set_property(TARGET optiga-trust-x PROPERTY C_STANDARD 99)

