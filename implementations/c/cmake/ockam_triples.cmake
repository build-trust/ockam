# Define functions to get the host and target triple.
#
# The triple has the general format <arch><sub>-<vendor>-<sys>-<abi>, where:
# arch = x86_64, i386, arm, thumb, mips, etc.
# sub = for ex. on ARM: v5, v6m, v7a, v7m, etc.
# vendor = pc, apple, nvidia, ibm, etc.
# sys = none, linux, win32, darwin, cuda, etc.
# abi = eabi, gnu, android, macho, elf, etc.

function(get_host_triple out out_arch out_vendor out_os out_abi)

  # Get the architecture.
  cmake_host_system_information(RESULT _os_platform QUERY OS_PLATFORM)
  message(STATUS "CMAKE_SYSTEM_OS_PLATFORM:      : '${_os_platform}'")

  if("${_os_platform}" STREQUAL "x86_64")
    set(arch "x86_64")
  elseif("${_os_platform}" STREQUAL "i686")
    set(arch "i686")
  else()
    set(arch "${_os_platform}")
  endif()

  # Get the vendor/os/abi
  cmake_host_system_information(RESULT _os_host_system QUERY OS_NAME)
  message(STATUS "CMAKE_SYSTEM_OS_NAME:          : '${_os_host_system}'")

  if ("${_os_host_system}" MATCHES "(Darwin|Mac OS X)")
    set(vendor "apple")
    set(os "darwin")
  elseif("${_os_host_system}" STREQUAL "Windows")
    set(vendor "pc")
    set(os "windows")
  elseif("${_os_host_system}" STREQUAL "Linux")
    set(vendor "unknown")
    set(os "linux")
    if("${_os_platform}" MATCHES "x86_64")
      set(abi "gnu")
    endif()
  else()
    message(FATAL_ERROR "Unknown operating system: ${_os_host_system}")
  endif()

  if(NOT DEFINED abi)
    set(triple "${arch}-${vendor}-${os}")
  else()
    set(triple "${arch}-${vendor}-${os}-${abi}")
  endif()
  set(${out} ${triple} PARENT_SCOPE)
  set(${out_arch} ${arch} PARENT_SCOPE)
  set(${out_vendor} ${vendor} PARENT_SCOPE)
  set(${out_os} ${os} PARENT_SCOPE)
  set(${out_abi} ${abi} PARENT_SCOPE)

endfunction()


function(get_target_triple out out_arch out_vendor out_os out_abi)
  if(DEFINED OCKAM_TARGET_TRIPLE)
    # Decompose the triple into its components
    set(triple "${OCKAM_TARGET_TRIPLE}")
    # Pop the arch, increment index past '-' for next segment
    string(FIND "${triple}" "-" _arch_end)
    string(SUBSTRING "${triple}" 0 ${_arch_end} arch)
    math(EXPR _arch_end "${_arch_end} + 1")
    string(SUBSTRING "${triple}" ${_arch_end} -1 _rest)
    # Pop the vendor, increment index past '-' for next segment
    string(FIND "${_rest}" "-" _vendor_end)
    string(SUBSTRING "${_rest}" 0 ${_vendor_end} vendor)
    math(EXPR _vendor_end "${_vendor_end} + 1")
    string(SUBSTRING "${_rest}" ${_vendor_end} -1 _rest)
    # Check to see if there is more than one segment left
    string(FIND "${_rest}" "-" _os_end)
    if (_os_end EQUAL -1)
      # There wasn't, so the remaining input is the OS
      set(os "${_rest}")
    else()
      # There was, so split on the '-' into OS and ABI, respectively
      string(SUBSTRING "${_rest}" 0 ${_os_end} os)
      math(EXPR _os_end "${_os_end} + 1")
      string(SUBSTRING "${_rest}" ${_os_end} -1 abi)
    endif()
  else()
    # Build the triple from its component parts
    set(arch "${OCKAM_TARGET_ARCHITECTURE}")
    set(vendor "${OCKAM_TARGET_VENDOR}")
    set(os "${OCKAM_TARGET_OS}")

    if(NOT DEFINED OCKAM_TARGET_ABI)
      set(triple "${arch}-${vendor}-${os}")
    else()
      set(abi "${OCKAM_TARGET_ABI}")
      set(triple "${arch}-${vendor}-${os}-${abi}")
    endif()
  endif()

  set(${out} ${triple} PARENT_SCOPE)
  set(${out_arch} ${arch} PARENT_SCOPE)
  set(${out_vendor} ${vendor} PARENT_SCOPE)
  set(${out_os} ${os} PARENT_SCOPE)
  set(${out_abi} ${abi} PARENT_SCOPE)

endfunction()
