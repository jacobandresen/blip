set(ASSET_STAMP ${CMAKE_CURRENT_BINARY_DIR}/assets.stamp)

add_custom_command(
    OUTPUT  ${ASSET_STAMP}
    COMMAND gcc ${CMAKE_CURRENT_SOURCE_DIR}/assets/generate_assets.c
                -o ${CMAKE_CURRENT_BINARY_DIR}/gen_assets -lm
    COMMAND ${CMAKE_CURRENT_BINARY_DIR}/gen_assets
    COMMAND ${CMAKE_COMMAND} -E touch ${ASSET_STAMP}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/assets
    COMMENT "Generating Serpent assets"
    VERBATIM
)
add_custom_target(serpent_assets DEPENDS ${ASSET_STAMP})

add_executable(serpent main.c)
add_dependencies(serpent serpent_assets)
target_link_libraries(serpent PRIVATE blip)
target_compile_options(serpent PRIVATE
    $<$<C_COMPILER_ID:GNU,Clang>:-Wall -Wextra -Wno-unused-parameter>)

add_custom_command(TARGET serpent POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${CMAKE_CURRENT_SOURCE_DIR}/assets
        $<TARGET_FILE_DIR:serpent>/assets
    COMMENT "Copying Serpent assets")
