#!/usr/bin/env perl
use warnings;
use strict;
use File::Temp qw/ tempfile /;

my ($crate, $ver) = @ARGV;

die "Missing arguments. Specify crate and version" unless $crate and $ver;

my ($tmp, $notes) = tempfile();

print $tmp qq{* [Crate](https://crates.io/crates/$crate/$ver)
* [Documentation](https://docs.rs/$crate/$ver/$crate/)
* [CHANGELOG](https://github.com/build-trust/ockam/blob/${crate}_v$ver/implementations/rust/ockam/$crate/CHANGELOG.md)
};

my $tag = $crate . "_v$ver";
my $title = qq{$crate v$ver (rust crate)};
my $cmd = qq{gh release create -F $notes -t "$title" $tag};
print `$cmd`;
