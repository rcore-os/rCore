// ".S" files: gedit needs this line to highlight the code

/*++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++}
{			variable init extracted from _start	                            }
{++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++*/

.section .text.UsbDependencyInit, "ax", %progbits
.balign	4
.globl UsbDependencyInit;
.type UsbDependencyInit, %function
UsbDependencyInit:
    ldr	x0, =0x3F000000						// No need to detect address in 64bit mode it has to be 0x3F000000 on Pi3
    ldr x1, =RPi_IO_Base_Addr				// Address of RPi_IO_Base_Addr
    str w0, [x1]							// Store the IO base address
    ret										// Return
.balign	4
.ltorg										/* Tell assembler ltorg data for code above can go here */
.size   UsbDependencyInit, .-UsbDependencyInit

/*++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++}
{			VC4 ADDRESS HELPER ROUTINES PROVIDE BY RPi-SmartStart API	    }
{++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++*/

/* "PROVIDE C FUNCTION: uint32_t ARMaddrToGPUaddr (void* ARMaddress);" */
.section .text.ARMaddrToGPUaddr, "ax", %progbits
.balign	4
.globl ARMaddrToGPUaddr;
.type ARMaddrToGPUaddr, %function
//"================================================================"
// ARMaddrToGPUaddr -- AARCH64 Pi3 code
// C Function: uint32_t ARMaddrToGPUaddr (void* ARMaddress);
// Entry: x0 will have ARMAddress value
//"================================================================"
ARMaddrToGPUaddr:
    mov x1, #0xC0000000						// ARM to VC conversion value
    orr x0, x0, x1							// Create bus address
    ret										// Return
.balign	4
.ltorg										// Tell assembler ltorg data for this code can go here
.size	ARMaddrToGPUaddr, .-ARMaddrToGPUaddr

/* "PROVIDE C FUNCTION: uint32_t GPUaddrToARMaddr (uint32_t BUSaddress);" */
.section .text.GPUaddrToARMaddr, "ax", %progbits
.balign	4
.globl GPUaddrToARMaddr;
.type GPUaddrToARMaddr, %function
//"================================================================"
// GPUaddrToARMaddr -- AARCH64 Pi3 code
// C Function: uint32_t GPUaddrToARMaddr (uint32_t BUSaddress);
// Entry: x0 will have GPUAddress value
//"================================================================"
GPUaddrToARMaddr:
    mov x1, ~0xC0000000						// ARM to VC conversion value	
    and x0, x0, x1							// Create arm address
    ret										// Return
.balign	4
.ltorg										// Tell assembler ltorg data for this code can go here
.size	GPUaddrToARMaddr, .-GPUaddrToARMaddr

/****************************************************************
              DATA FOR SMARTSTART64  EXPOSED TO INTERFACE 
****************************************************************/
.section ".data.usb", "aw"
.balign 4

.globl RPi_IO_Base_Addr;				// Make sure Pi_IO_Base_Addr label is global
RPi_IO_Base_Addr : .4byte 0;			// Peripheral Base addr is 4 byte variable in 64bit mode

