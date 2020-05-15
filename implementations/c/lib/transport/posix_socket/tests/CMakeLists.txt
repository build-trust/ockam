# Copy test fixtures to the right plac

set(OCKAM_TRANSPORT_TEST_FIXTURE_DIR ${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}/tests/fixtures)
set(_fixtures_src_dir ${CMAKE_CURRENT_SOURCE_DIR}/fixtures)

file(GLOB _src_fixtures LIST_DIRECTORIES false "${_fixtures_src_dir}/*.txt")
file(MAKE_DIRECTORY ${OCKAM_TRANSPORT_TEST_FIXTURE_DIR})
foreach(_src ${_src_fixtures})
	string(FIND ${_src} "/" _idx REVERSE)
	math(EXPR _idx1 "${_idx} + 1")
	string(SUBSTRING ${_src} ${_idx1} -1 _filename)
	configure_file(${_src} ${OCKAM_TRANSPORT_TEST_FIXTURE_DIR}/${_filename} )
endforeach()

install(
		DIRECTORY ${_fixtures_src_dir}
		DESTINATION ${OCKAM_TESTS_OUTPUT_DIRECTORY}
		FILES_MATCHING PATTERN "*.txt*"
)

add_subdirectory(posix_socket)
