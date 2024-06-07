#!/usr/bin/expect

spawn /tmp/pbo-assistant/mprime

expect "Your choice"
send "16\r"

expect "Number of cores to torture test"
send "1\r"

expect "Use hyperthreading"
send "N\r"

expect "Type of torture test to run"
send "2\r"

expect "Customize settings"
send "N\r"

expect "Run a weaker torture test"
send "N\r"

expect "Accept the answers above"
send "Y\r"

# Interact with the program if needed
interact
