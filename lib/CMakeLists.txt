add_library(blip STATIC blip.c)

target_include_directories(blip PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

target_link_libraries(blip PUBLIC SDL3::SDL3 m)

target_compile_options(blip PRIVATE
    $<$<C_COMPILER_ID:GNU,Clang>:-Wall -Wextra -Wno-unused-parameter>)
