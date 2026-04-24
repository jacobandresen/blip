set(ASSET_STAMP ${CMAKE_CURRENT_BINARY_DIR}/assets.stamp)

add_custom_command(
    OUTPUT  ${ASSET_STAMP}
    COMMAND gcc ${CMAKE_CURRENT_SOURCE_DIR}/assets/generate_assets.c
                -o ${CMAKE_CURRENT_BINARY_DIR}/gen_assets -lm
    COMMAND ${CMAKE_CURRENT_BINARY_DIR}/gen_assets
    COMMAND ${CMAKE_COMMAND} -E touch ${ASSET_STAMP}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/assets
    COMMENT "Generating Galactic Defender assets"
    VERBATIM
)
add_custom_target(galactic_assets DEPENDS ${ASSET_STAMP})

add_executable(galactic_defender main.c)
add_dependencies(galactic_defender galactic_assets)
target_link_libraries(galactic_defender PRIVATE blip)
target_compile_options(galactic_defender PRIVATE
    $<$<C_COMPILER_ID:GNU,Clang>:-Wall -Wextra -Wno-unused-parameter>)

add_custom_command(TARGET galactic_defender POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${CMAKE_CURRENT_SOURCE_DIR}/assets
        $<TARGET_FILE_DIR:galactic_defender>/assets
    COMMENT "Copying Galactic Defender assets")
