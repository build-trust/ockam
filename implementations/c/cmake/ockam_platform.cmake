# Supported target triples (case-sensitive):
#   Mac: x86_64-apple-darwin        "-DOCKAM_TARGET_PLATFORM=mac"
#   Raspberry Pi: arm-raspi-linux   "-DOCKAM_TARGET_PLATFORM=raspi"

# Set up host and target architectures (default is mac)
if(DEFINED OCKAM_TARGET_PLATFORM)
  if(${OCKAM_TARGET_PLATFORM} STREQUAL "pi")
    include(${CMAKE_MODULE_PATH}/platform/raspberry-pi.cmake)
    set(CMAKE_C_COMPILER_ID "GNU")
    set(CMAKE_CXX_COMPILER_ID "GNU")
  elseif(${OCKAM_TARGET_PLATFORM} STREQUAL "pi-mac")
    include(${CMAKE_MODULE_PATH}/platform/raspberry-pi-mac.cmake)
    set(CMAKE_C_COMPILER_ID "GNU")
    set(CMAKE_CXX_COMPILER_ID "GNU")
  elseif(${OCKAM_TARGET_PLATFORM} STREQUAL "mac")
    include(${CMAKE_MODULE_PATH}/platform/mac.cmake)
    set(CMAKE_C_COMPILER_ID "GNU")
    set(CMAKE_CXX_COMPILER_ID "GNU")
  elseif(${OCKAM_TARGET_PLATFORM} STREQUAL "linux")
    include(${CMAKE_MODULE_PATH}/platform/linux.cmake)
    #!! For some reason, CMAKE_<LANG>_COMPILER_ID is not being set. For now,
    #!! set it to "GNU", since that is what we build with on mac
    set(CMAKE_C_COMPILER_ID "GNU")
    set(CMAKE_CXX_COMPILER_ID "GNU")
  endif()
else()
  message(STATUS "+++++++++++++++++++TARGET PLATFORM UNDEFINED")
endif()
