#include <stdint.h>
#include <stdarg.h>
#include "usb-dependency.h"

void memset(void *d, int c, size_t n)
{
    register char *o=(char *)d, *p=o+n;
    for(;o!=p;++o)
        *o=(char)c;
}
void memcpy(void *d, const void *s, size_t n)
{
    //if((unsigned long long)d<(unsigned long long)s)
    if(1)
    { // real memcpy
        register char *o=(char *)d, *p=o+n;
        register char *oo=(char *)s;
        for(;o!=p;++o,++oo)
            *o=*oo;
    }
    else
    { //memmove(d, s, n), memcpy=memmove in VS
        register char *o=(char *)d+(n-1), *p=o-n;
        register char *oo=(char *)s+(n-1);
        for(;o!=p;--o,--oo)
            *o=*oo;
    }
}

/***************************************************************************}
{       PRIVATE INTERNAL RASPBERRY PI REGISTER STRUCTURE DEFINITIONS        }
****************************************************************************/

/*--------------------------------------------------------------------------}
{  RASPBERRY PI SYSTEM TIMER HARDWARE REGISTERS - BCM2835 Manual Section 12	}
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) SystemTimerRegisters {
    uint32_t ControlStatus;											// 0x00
    uint32_t TimerLo;												// 0x04
    uint32_t TimerHi;												// 0x08
    uint32_t Compare0;												// 0x0C
    uint32_t Compare1;												// 0x10
    uint32_t Compare2;												// 0x14
    uint32_t Compare3;												// 0x18
};

/*--------------------------------------------------------------------------}
;{               RASPBERRY PI MAILBOX HARRDWARE REGISTERS					}
;{-------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) MailBoxRegisters {
    const uint32_t Read0;											// 0x00         Read data from VC to ARM
    uint32_t Unused[3];												// 0x04-0x0F
    uint32_t Peek0;													// 0x10
    uint32_t Sender0;												// 0x14
    uint32_t Status0;												// 0x18         Status of VC to ARM
    uint32_t Config0;												// 0x1C        
    uint32_t Write1;												// 0x20         Write data from ARM to VC
    uint32_t Unused2[3];											// 0x24-0x2F
    uint32_t Peek1;													// 0x30
    uint32_t Sender1;												// 0x34
    uint32_t Status1;												// 0x38         Status of ARM to VC
    uint32_t Config1;												// 0x3C 
};

/***************************************************************************}
{     PRIVATE POINTERS TO ALL OUR RASPBERRY PI REGISTER BANK STRUCTURES	    }
****************************************************************************/
//#define GPIO ((volatile __attribute__((aligned(4))) struct GPIORegisters*)(uintptr_t)(RPi_IO_Base_Addr + 0x200000))
#define SYSTEMTIMER ((volatile __attribute__((aligned(4))) struct SystemTimerRegisters*)(uintptr_t)(RPi_IO_Base_Addr + 0x3000))
//#define IRQ ((volatile __attribute__((aligned(4))) struct IrqControlRegisters*)(uintptr_t)(RPi_IO_Base_Addr + 0xB200))
//#define ARMTIMER ((volatile __attribute__((aligned(4))) struct ArmTimerRegisters*)(uintptr_t)(RPi_IO_Base_Addr + 0xB400))
#define MAILBOX ((volatile __attribute__((aligned(4))) struct MailBoxRegisters*)(uintptr_t)(RPi_IO_Base_Addr + 0xB880))

/*==========================================================================}
{		   PUBLIC TIMER ROUTINES PROVIDED BY RPi-SmartStart API				}
{==========================================================================*/

/*-[timer_getTickCount64]---------------------------------------------------}
. Get 1Mhz ARM system timer tick count in full 64 bit.
. The timer read is as per the Broadcom specification of two 32bit reads
. RETURN: tickcount value as an unsigned 64bit value in microseconds (usec)
. 30Jun17 LdB
.--------------------------------------------------------------------------*/
uint64_t timer_getTickCount64(void)
{
    uint64_t resVal;
    uint32_t lowCount;
    do {
        resVal = SYSTEMTIMER->TimerHi; 								// Read Arm system timer high count
        lowCount = SYSTEMTIMER->TimerLo;							// Read Arm system timer low count
    } while (resVal != (uint64_t)SYSTEMTIMER->TimerHi);				// Check hi counter hasn't rolled in that time
    resVal = (uint64_t)resVal << 32 | lowCount;						// Join the 32 bit values to a full 64 bit
    return(resVal);													// Return the uint64_t timer tick count
}

/*-[timer_Wait]-------------------------------------------------------------}
. This will simply wait the requested number of microseconds before return.
. 02Jul17 LdB
.--------------------------------------------------------------------------*/
void timer_wait (uint64_t us) 
{
    us += timer_getTickCount64();									// Add current tickcount onto delay
    while (timer_getTickCount64() < us) {};							// Loop on timeout function until timeout
}

/*-[tick_Difference]--------------------------------------------------------}
. Given two timer tick results it returns the time difference between them.
. 02Jul17 LdB
.--------------------------------------------------------------------------*/
uint64_t tick_difference (uint64_t us1, uint64_t us2) 
{
    if (us1 > us2) {												// If timer one is greater than two then timer rolled
        uint64_t td = UINT64_MAX - us1 + 1;							// Counts left to roll value
        return us2 + td;											// Add that to new count
    }
    return us2 - us1;												// Return difference between values
}

/*==========================================================================}
{		  PUBLIC PI MAILBOX ROUTINES PROVIDED BY RPi-SmartStart API			}
{==========================================================================*/
#define MAIL_EMPTY	0x40000000		/* Mailbox Status Register: Mailbox Empty */
#define MAIL_FULL	0x80000000		/* Mailbox Status Register: Mailbox Full  */

/*-[mailbox_write]----------------------------------------------------------}
. This will execute the sending of the given data block message thru the
. mailbox system on the given channel.
. RETURN: True for success, False for failure.
. 04Jul17 LdB
.--------------------------------------------------------------------------*/
bool mailbox_write (MAILBOX_CHANNEL channel, uint32_t message) 
{
    uint32_t value;													// Temporary read value
    if (channel > MB_CHANNEL_GPU)  return false;					// Channel error
    message &= ~(0xF);												// Make sure 4 low channel bits are clear 
    message |= channel;												// OR the channel bits to the value							
    do {
        value = MAILBOX->Status1;									// Read mailbox1 status from GPU	
    } while ((value & MAIL_FULL) != 0);								// Make sure arm mailbox is not full
    MAILBOX->Write1 = message;										// Write value to mailbox
    return true;													// Write success
}

/*-[mailbox_read]-----------------------------------------------------------}
. This will read any pending data on the mailbox system on the given channel.
. RETURN: The read value for success, 0xFEEDDEAD for failure.
. 04Jul17 LdB
.--------------------------------------------------------------------------*/
uint32_t mailbox_read (MAILBOX_CHANNEL channel) 
{
    uint32_t value;													// Temporary read value
    if (channel > MB_CHANNEL_GPU)  return 0xFEEDDEAD;				// Channel error
    do {
        do {
            value = MAILBOX->Status0;								// Read mailbox0 status
        } while ((value & MAIL_EMPTY) != 0);						// Wait for data in mailbox
        value = MAILBOX->Read0;										// Read the mailbox	
    } while ((value & 0xF) != channel);								// We have response back
    value &= ~(0xF);												// Lower 4 low channel bits are not part of message
    return value;													// Return the value
}

/*-[mailbox_tag_message]----------------------------------------------------}
. This will post and execute the given variadic data onto the tags channel
. on the mailbox system. You must provide the correct number of response
. uint32_t variables and a pointer to the response buffer. You nominate the
. number of data uint32_t for the call and fill the variadic data in. If you
. do not want the response data back the use NULL for response_buf pointer.
. RETURN: True for success and the response data will be set with data
.         False for failure and the response buffer is untouched.
. 04Jul17 LdB
.--------------------------------------------------------------------------*/
bool mailbox_tag_message (uint32_t* response_buf,					// Pointer to response buffer 
                          uint8_t data_count,						// Number of uint32_t data following
                          ...)										// Variadic uint32_t values for call
{
    uint32_t __attribute__((aligned(16))) message[32];
    va_list list;
    va_start(list, data_count);										// Start variadic argument
    message[0] = (data_count + 3) * 4;								// Size of message needed
    message[data_count + 2] = 0;									// Set end pointer to zero
    message[1] = 0;													// Zero response message
    for (int i = 0; i < data_count; i++) {
        message[2 + i] = va_arg(list, uint32_t);					// Fetch next variadic
    }
    va_end(list);													// variadic cleanup								
    mailbox_write(MB_CHANNEL_TAGS, ARMaddrToGPUaddr(&message[0]));	// Write message to mailbox
    mailbox_read(MB_CHANNEL_TAGS);									// Wait for write response	
    if (message[1] == 0x80000000) {
        if (response_buf) {											// If buffer NULL used then don't want response
            for (int i = 0; i < data_count; i++)
                response_buf[i] = message[2 + i];					// Transfer out each response message
        }
        return true;												// message success
    }
    return false;													// Message failed
}

