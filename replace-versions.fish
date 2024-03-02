#!/usr/bin/env fish

set regex '\d+\.\d+\.\d+\+\w+\+chromium-\d+\.\d+\.\d+\.\d+'

set from (cat src/cef_binary_updater.rs | rg -o $regex |head -n1)
set to $argv

sd --fixed-strings $from $to src/cef_binary_updater.rs
