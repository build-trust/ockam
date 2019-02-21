#!/bin/bash
# create private key
openssl genrsa -out keypair.pem 2048
# generate public key from private key
openssl rsa -in keypair.pem -pubout -out publickey.crt
# convert keypair to pkcs8
openssl pkcs8 -topk8 -inform PEM -outform PEM -nocrypt -in keypair.pem -out pkcs8.key
