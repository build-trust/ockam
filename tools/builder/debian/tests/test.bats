#!/usr/bin/env bats

@test "has bash 4.4.12" {
e='GNU bash, version 4.4.12(1)-release (x86_64-pc-linux-gnu)
Copyright (C) 2016 Free Software Foundation, Inc.
License GPLv3+: GNU GPL version 3 or later <http://gnu.org/licenses/gpl.html>

This is free software; you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.'
o="$(bash --version)"
[ "$e" == "$o" ]
}

@test "has bats 1.1.0" {
e='Bats 1.1.0'
o="$(bats --version)"
[ "$e" == "$o" ]
}

@test "has shellcheck 0.6.0" {
e='version: 0.6.0'
o="$(shellcheck --version | grep "version:")"
[ "$e" == "$o" ]
}

@test "has g++ 6.3.0" {
e='g++ (Debian 6.3.0-18+deb9u1) 6.3.0 20170516
Copyright (C) 2016 Free Software Foundation, Inc.
This is free software; see the source for copying conditions.  There is NO
warranty; not even for MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.'
o="$(g++ --version)"
[ "$e" == "$o" ]
}

@test "has gcc 6.3.0" {
e='gcc (Debian 6.3.0-18+deb9u1) 6.3.0 20170516
Copyright (C) 2016 Free Software Foundation, Inc.
This is free software; see the source for copying conditions.  There is NO
warranty; not even for MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.'
o="$(gcc --version)"
[ "$e" == "$o" ]
}

@test "has cmake 3.14.5" {
e='cmake version 3.14.5

CMake suite maintained and supported by Kitware (kitware.com/cmake).'
o="$(cmake --version)"
[ "$e" == "$o" ]
}

@test "has make 4.1" {
e='GNU Make 4.1
Built for x86_64-pc-linux-gnu
Copyright (C) 1988-2014 Free Software Foundation, Inc.
License GPLv3+: GNU GPL version 3 or later <http://gnu.org/licenses/gpl.html>
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.'
o="$(make --version)"
[ "$e" == "$o" ]
}

@test "has erlang otp 22.2.3" {
e='22.2.3'
o="$(cat $(dirname $(dirname `which erl`)/$(readlink `which erl`))/../releases/*/OTP_*)"
[ "$e" == "$o" ]
}

@test "has elixir 1.10.0" {
e='Elixir 1.10.0 (compiled with Erlang/OTP 22)'
o="$(elixir --version | grep Elixir)"
[ "$e" == "$o" ]
}

@test "has iex 1.10.0" {
e='IEx 1.10.0 (compiled with Erlang/OTP 22)'
o="$(iex --version | grep IEx)"
[ "$e" == "$o" ]
}

@test "has mix 1.10.0" {
e='Mix 1.10.0 (compiled with Erlang/OTP 22)'
o="$(mix --version | grep Mix)"
[ "$e" == "$o" ]
}

@test "has openjdk 11.0.6" {
e='openjdk version "11.0.6" 2020-01-14
OpenJDK Runtime Environment AdoptOpenJDK (build 11.0.6+10)
OpenJDK 64-Bit Server VM AdoptOpenJDK (build 11.0.6+10, mixed mode)'
o="$(java -version 2>&1)"
[ "$e" == "$o" ]
}

@test "has javac 11.0.6" {
e='javac 11.0.6'
o="$(javac -version 2>&1)"
[ "$e" == "$o" ]
}

@test "has node 10.16.0" {
e='v10.16.0'
o="$(node --version)"
[ "$e" == "$o" ]
}

@test "has npm 6.9.0" {
e='6.9.0'
o="$(npm --version)"
[ "$e" == "$o" ]
}

@test "has yarn 1.16.0" {
e='1.16.0'
o="$(yarn --version)"
[ "$e" == "$o" ]
}

@test "has go 1.12.6" {
e='go version go1.12.6 linux/amd64'
o="$(go version)"
[ "$e" == "$o" ]
}

@test "has cargo 1.42.0" {
e='cargo 1.42.0 (86334295e 2020-01-31)'
o="$(cargo --version)"
[ "$e" == "$o" ]
}

@test "has rustc 1.42.0" {
e='rustc 1.42.0 (b8cedc004 2020-03-09)'
o="$(rustc --version)"
[ "$e" == "$o" ]
}

@test "has rustup 1.21.1" {
e='rustup 1.21.1 (7832b2ebe 2019-12-20)'
o="$(rustup --version)"
[ "$e" == "$o" ]
}
