cmake_minimum_required(VERSION 3.20)
project(GameProgramming C)
set(CMAKE_C_STANDARD 11)

find_package(SDL3 REQUIRED CONFIG)

add_subdirectory(lib)
add_subdirectory(games/serpent)
add_subdirectory(games/galactic_defender)
add_subdirectory(games/bouncer)
add_subdirectory(games/rally)
