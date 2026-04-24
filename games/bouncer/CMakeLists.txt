set(ASSET_STAMP ${CMAKE_CURRENT_BINARY_DIR}/assets.stamp)

add_custom_command(
    OUTPUT  ${ASSET_STAMP}
    COMMAND gcc ${CMAKE_CURRENT_SOURCE_DIR}/assets/generate_assets.c
                -o ${CMAKE_CURRENT_BINARY_DIR}/gen_assets -lm
    COMMAND ${CMAKE_CURRENT_BINARY_DIR}/gen_assets
    COMMAND ${CMAKE_COMMAND} -E touch ${ASSET_STAMP}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/assets
    COMMENT "Generating Bouncer assets"
    VERBATIM
)
add_custom_target(bouncer_assets DEPENDS ${ASSET_STAMP})

add_executable(bouncer main.c)
add_dependencies(bouncer bouncer_assets)
target_link_libraries(bouncer PRIVATE blip)
target_compile_options(bouncer PRIVATE
    $<$<C_COMPILER_ID:GNU,Clang>:-Wall -Wextra -Wno-unused-parameter>)

add_custom_command(TARGET bouncer POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${CMAKE_CURRENT_SOURCE_DIR}/assets
        $<TARGET_FILE_DIR:bouncer>/assets
    COMMENT "Copying Bouncer assets")
