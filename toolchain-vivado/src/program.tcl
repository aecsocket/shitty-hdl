# init args
if {$argc < 1} {
    error "usage: program.tcl BITSTREAM ?PART?"
}
lassign $argv bitstream part
set bitstream [file normalize $bitstream]

if {![file exists $bitstream]} {
    error "bitstream not found: $bitstream"
}

# connect to the board

open_hw_manager
connect_hw_server
open_hw_target

set devices [get_hw_devices -quiet]
if {[llength $devices] == 0} {
    error "no FPGA devices found on the connected targets"
}

if {$part ne ""} {
    set matching {}
    foreach device $devices {
        set name [get_property NAME $device]
        if {![regexp {^(.*)_([0-9]+)$} $name -> device_base]} {
            set device_base $name
        }
        if {[string first $device_base $part] == 0
            || [get_property PART $device] eq $part} {
            lappend matching $device
        }
    }
    if {[llength $matching] == 0} {
        error "no device with part $part found; found: $devices"
    }
    set devices $matching
}
set device [lindex $devices 0]

# program the device

set_property PROBES.FILE {} $device
set_property FULL_PROBES.FILE {} $device
set_property PROGRAM.FILE $bitstream $device
program_hw_devices $device

close_hw_target
disconnect_hw_server
close_hw_manager

flush stdout
