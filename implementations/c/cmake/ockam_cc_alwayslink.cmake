include(CMakeParseArguments)

# Additional libraries containing statically registered functions/flags, which
# should always be linked in to binaries.


# set_alwayslink_property()
#
# CMake function to set the ALWAYSLINK on external libraries
#
# Parameters:
# ALWAYSLINK_LIBS: List of libraries
# SKIP_NONEXISTING: When added, ALWAYSLINK is only set on existing libraries.

function(set_alwayslink_property)
  cmake_parse_arguments(
    _RULE
    "SKIP_NONEXISTING"
    ""
    "ALWAYSLINK_LIBS"
    ${ARGN}
  )

  foreach(_LIB ${_RULE_ALWAYSLINK_LIBS})
    # If SKIP_NONEXISTING is false: Always try to set the property.
    # If SKIP_NONEXISTING is true : Only set the property if the target exists.
    message(STATUS "Processing _LIB         : '${_LIB}'")
    if(NOT TARGET ${_LIB} AND _RULE_SKIP_NONEXISTING)
      continue()
    endif()

    # Check if the target is an aliased target.
    # If so get the non aliased target.
    get_target_property(_ALIASED_TARGET ${_LIB} ALIASED_TARGET)
    if(_ALIASED_TARGET)
      set(_LIB ${_ALIASED_TARGET})
    endif()

    set_property(TARGET ${_LIB} PROPERTY ALWAYSLINK 1)
  endforeach()
endfunction()


function(set_alwayslink_ockam_libs)
  set(_ALWAYSLINK_LIBS_OCKAM "")
  #set(_ALWAYSLINK_LIBS_OCKAM
  #  some_dependency
  #)
  message(STATUS "----------- set_alwayslink_ockam_libs ------------")
  set_alwayslink_property(
    ALWAYSLINK_LIBS
      ${_ALWAYSLINK_LIBS_OCKAM}
  )
endfunction()

function(set_alwayslink_ockam_includes)
  file(COPY ${PROJECT_SOURCE_DIR}/include/
    DESTINATION ${OCKAM_INCLUDES_OUTPUT_DIRECTORY}
  )
endfunction()
