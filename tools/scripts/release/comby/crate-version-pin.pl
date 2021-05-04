#!/usr/bin/perl
use strict;
use warnings;


while(my $line = <STDIN>) {
  my($crate, $vers) = $line =~ qr{([\w\d_]+)\s+([\w\d.-]+)};
  print "[$crate]\n";
  print qq{match='$crate = { path = ":[path]", version = ":[_]" }'\n};
  print qq{rewrite='$crate = { path = ":[path]", version = "$vers" }'\n};
  print "[$crate-opt]\n";
  print qq{match='$crate = { path = ":[path]", version = ":[_]", optional = :[opt] }'\n};
  print qq{rewrite='$crate = { path = ":[path]", version = "$vers", optional = :[opt] }'\n};
  print "\n";
}
