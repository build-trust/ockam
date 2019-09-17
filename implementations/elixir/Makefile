
CFLAGS = -g -O3 -ansi -pedantic -Wall -Wextra -Wno-unused-parameter

ERLANG_PATH = $(shell erl -eval 'io:format("~s", [lists:concat([code:root_dir(), "/erts-", erlang:system_info(version), "/include"])])' -s init stop -noshell)
CFLAGS += -I$(ERLANG_PATH) -I../c/include -I../c/src
CFLAGS += -Isrc


.PHONY: all clean

all: priv/ockam.so

priv/ockam.so: src/ockam.c
	mkdir -p priv
	$(CC) $(CFLAGS) -shared $(LDFLAGS) -o $@ src/ockam.c

clean:
	$(RM) -r priv/ockam.so*
