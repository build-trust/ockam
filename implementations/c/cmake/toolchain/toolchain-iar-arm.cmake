####
#
# This file is seeded and adapted from IAR Systems 'technical note 1'
#
# Technical note 1: https://www.iar.com/support/tech-notes/general/using-cmake-with-iar-embedded-workbench
#
# Technical note 2: https://www.iar.com/support/tech-notes/debugger/debugging-an-externally-built-executable-file/

# 'Technical note 2' offers valuable guidance on how to set up "debug only" projects that can load
# your cmake- (or more generically externally-) built artifacts.
#
# If you desire to use IAR EW as native IDE, you'll need to create EW project and then manually add
# every file using the IDE GUI. To maintain folder structure, use "add group" to mimic/replicate the folder
# hierarchy. The project config/structure is eventually saved into XML files. So presumably, you could
# also create by hand (or write a script to generate) the XML files.
#
# Note that the ockam c implementation has external library dependencies for which source code is checked
# out by cmake at build time, these dependencies will need to be resolved to achieve the desired end state of
# full native IDE usage for this project.
#
# IAR C/C++ Dev Guide: http://ftp.iar.se/WWWfiles/arm/webic/doc/EWARM_DevelopmentGuide.ENU.pdf
#
# How to build ockam for IAR using cmake:
#
#  Windows (on cmd prompt):
#     Option #1 (MinGW):
#       1. cd <ockam_repo clone>\implementations\c
#       2. cmake -G "MinGW Makefiles" -DCMAKE_TOOLCHAIN_FILE=cmake/toolchain/toolchain-iar-arm.cmake -S . -B build
#       3. mingw32-make
#
#       * You need to have MinGW installed and in system path env variable.
#
#     Option #2 (Nmake):
##      1. cd <ockam_repo clone>\implementations\c
#       2. cmake -G "NMake Makefiles" -DCMAKE_TOOLCHAIN_FILE=cmake/toolchain/toolchain-iar-arm.cmake -S . -B build
#       3. nmake
#
#       * Nmake comes with Visual Studio installs and Visual C++ tools installs.
#
#  Linux (default shell):
#     1. cd <ockam_repo clone>\implementations\c
#     2. cmake -G "Unix Makefiles" -DCMAKE_TOOLCHAIN_FILE=cmake/toolchain/toolchain-iar-arm.cmake -S . -B build
#     3. make
#
#     * This should also be doable on Windows Linux Subsystem (WSL) assuming you have IAR toolchain there
#       but I have not tested it.

# DISCLAIMER:
# This setup is not specific to any board/system, but rather a reference build that
# it can be done assuming the user has knowledge of (and performs updates to) the CPU, desired cpu options,
# and icf linker config file. It also assumes the user has a valid IAR EW license and IAR installed.
#
####

# "Generic" is used when cross-compiling.
set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR "ARM")

set(CMAKE_CROSSCOMPILING TRUE)

# Set the IAR embedded workbench installation root directory.
set(IAR_EW_ROOT_DIR "C:/Program Files (x86)/IAR Systems/Embedded Workbench 8.2/arm")

# Compiler config.

# I picked Cortex-M33 because it's a V8-M CPU, and therefore supports TF-M (ARM TrustZone).
# Hopefully, this helps someone later pick up that effort.

# cpu_mode: a/arm or t/thumb ISA.
set(CMAKE_C_COMPILER "${IAR_EW_ROOT_DIR}/bin/iccarm.exe" "--cpu=Cortex-M33 --cpu_mode=t --dlib_config=normal")
set(CMAKE_CXX_COMPILER "${IAR_EW_ROOT_DIR}/bin/iccarm.exe" "--cpu=Cortex-M33 --cpu_mode=t --dlib_config=normal")
set(CMAKE_ASM_COMPILER "${IAR_EW_ROOT_DIR}/bin/iasmarm.exe" "--cpu=Cortex-M33 --cpu_mode=t")

# Linker config.

#set(LINKER_SCRIPT "C:/Users/<username>/Desktop/Musca_A1_CPU0.icf")

# I had to use the path above (commented out) for local building/testing because IAR linker was having
# issues finding the config file and failing the linking cmake test. The issue was root caused to
# whitespaces in the directory path. So avoid them, if you can, on your setup.

# I picked 'Musca_A1_CPU0.icf' because (1) it was easy to work with (2) compatible with Cortext-M33
# (3) a reference implementation of an Arm TrustZone system.
# https://www.arm.com/products/development-tools/development-boards/musca-a1-iot
set(LINKER_SCRIPT "${IAR_ROOT_DIR}/config/linker/ARM/Musca_A1_CPU0.icf")
set(CMAKE_C_LINK_FLAGS "--config=${LINKER_SCRIPT}")
