/******************************************************************************
  Complete redux of CSUD (Chadderz's Simple USB Driver) by Alex Chadwick
  by Leon de Boer(LdB) 2017, 2018

  CSUD was overly complex in both it's coding and it's implementation for what
  it actually did. At it's heart CSUD simply provides the CONTROL pipe operation
  of a USB bus. That provides all the functionality to enumerate the USB bus 
  and control devices on the BUS.

*******************************************************************************/
#include <stdbool.h>			// C standard needed for bool
#include <stdlib.h>				// C standard needed for NULL
#include <stdint.h>				// C standard needed for uint8_t, uint32_t, uint64_t etc
#include <string.h>				// C standard needed for memset
#include <wchar.h>				// C standard needed for UTF for unicode descriptor support
#include "rpi-smartstart.h"		// Provides timing routines and mailbox routines to power up/down the USB.  
#include "rpi-usb.h"			// This units header

#define ReceiveFifoSize 20480 /* 16 to 32768 */
#define NonPeriodicFifoSize 20480 /* 16 to 32768 */
#define PeriodicFifoSize 20480 /* 16 to 32768 */

#define ControlMessageTimeout 10

/***************************************************************************}
{						 PRIVATE INTERNAL ENUMERATIONS					    }
****************************************************************************/

/*--------------------------------------------------------------------------}
{	       FLUSH TYPE ENUMERATION FOR FIFO ON THE DESIGNWARE 2.0		    }
{--------------------------------------------------------------------------*/
enum CoreFifoFlush {
	FlushNonPeriodic = 0,
	FlushPeriodic1 = 1,
	FlushPeriodic2 = 2,
	FlushPeriodic3 = 3,
	FlushPeriodic4 = 4,
	FlushPeriodic5 = 5,
	FlushPeriodic6 = 6,
	FlushPeriodic7 = 7,
	FlushPeriodic8 = 8,
	FlushPeriodic9 = 9,
	FlushPeriodic10 = 10,
	FlushPeriodic11 = 11,
	FlushPeriodic12 = 12,
	FlushPeriodic13 = 13,
	FlushPeriodic14 = 14,
	FlushPeriodic15 = 15,
	FlushAll = 16,
};

/***************************************************************************}
{         PRIVATE INTERNAL DESIGNWARE 2.0 CORE REGISTER STRUCTURES          }
****************************************************************************/

/*--------------------------------------------------------------------------}
{	      INTERRUPT BITS ON THE USB CHANNELS ON THE DESIGNWARE 2.0		    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) ChannelInterrupts {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile bool TransferComplete : 1;						// @0
			volatile bool Halt : 1;									// @1
			volatile bool AhbError : 1;								// @2
			volatile bool Stall : 1;								// @3
			volatile bool NegativeAcknowledgement : 1;				// @4
			volatile bool Acknowledgement : 1;						// @5
			volatile bool NotYet : 1;								// @6
			volatile bool TransactionError : 1;						// @7
			volatile bool BabbleError : 1;							// @8
			volatile bool FrameOverrun : 1;							// @9
			volatile bool DataToggleError : 1;						// @10
			volatile bool BufferNotAvailable : 1;					// @11
			volatile bool ExcessiveTransmission : 1;				// @12
			volatile bool FrameListRollover : 1;					// @13
			unsigned _reserved14_31 : 18;							// @14-31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{	   FIFOSIZE STRUCTURE .. THERE ARE A FEW OF THESE ON DESIGNWARE 2.0     }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) FifoSize {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile unsigned StartAddress : 16;					// @0
			volatile unsigned Depth : 16;							// @16
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{					   USB CORE OTG CONTROL STRUCTURE					    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) CoreOtgControl {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile bool sesreqscs : 1;							// @0
			volatile bool sesreq : 1;								// @1
			volatile bool vbvalidoven : 1;							// @2
			volatile bool vbvalidovval : 1;							// @3
			volatile bool avalidoven : 1;							// @4
			volatile bool avalidovval : 1;							// @5
			volatile bool bvalidoven : 1;							// @6
			volatile bool bvalidovval : 1;							// @7
			volatile bool hstnegscs : 1;							// @8
			volatile bool hnpreq : 1;								// @9
			volatile bool HostSetHnpEnable : 1;						// @10
			volatile bool devhnpen : 1;								// @11
			volatile unsigned _reserved12_15 : 4;					// @12-15
			volatile bool conidsts : 1;								// @16
			volatile unsigned dbnctime : 1;							// @17
			volatile bool ASessionValid : 1;						// @18
			volatile bool BSessionValid : 1;						// @19
			volatile unsigned OtgVersion : 1;						// @20
			volatile unsigned _reserved21 : 1;						// @21
			volatile unsigned multvalidbc : 5;						// @22-26
			volatile bool chirpen : 1;								// @27
			volatile unsigned _reserved28_31 : 4;					// @28-31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{					 USB CORE OTG INTERRUPT STRUCTURE					    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) CoreOtgInterrupt {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile unsigned _reserved0_1 : 2;						// @0
			volatile bool SessionEndDetected : 1;					// @2
			volatile unsigned _reserved3_7 : 5;						// @3
			volatile bool SessionRequestSuccessStatusChange : 1;	// @8
			volatile bool HostNegotiationSuccessStatusChange : 1;	// @9
			volatile unsigned _reserved10_16 : 7;					// @10
			volatile bool HostNegotiationDetected : 1;				// @17
			volatile bool ADeviceTimeoutChange : 1;					// @18
			volatile bool DebounceDone : 1;							// @19
			volatile unsigned _reserved20_31 : 12;					// @20-31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{	 USB CORE AHB STRUCTURE ... CARE WRITE WHOLE REGISTER .. NO BIT OPS	    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) CoreAhb {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile bool InterruptEnable : 1;						// @0
			volatile enum {
				Length4 = 0,
				Length3 = 1,
				Length2 = 2,
				Length1 = 3,
			} AxiBurstLength : 2;									// @1
			volatile unsigned _reserved3 : 1;						// @3
			volatile bool WaitForAxiWrites : 1;						// @4
			volatile bool DmaEnable : 1;							// @5
			volatile unsigned _reserved6 : 1;						// @6
			volatile enum EmptyLevel {
				Empty = 1,
				Half = 0,
			} TransferEmptyLevel : 1;								// @7
			volatile enum EmptyLevel PeriodicTransferEmptyLevel : 1;// @8
			volatile unsigned _reserved9_20 : 12;					// @9
			volatile bool remmemsupp : 1;							// @21
			volatile bool notialldmawrit : 1;						// @22
			volatile enum {
				Incremental = 0,
				Single = 1, // (default)
			} DmaRemainderMode : 1;									// @23
			volatile unsigned _reserved24_31 : 8;					// @24-31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{	USB CORE CONTROL STRUCTURE	.. CARE WRITE WHOLE REGISTER .. NO BIT OPS  }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) UsbControl {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile unsigned toutcal : 3;							// @0
			volatile bool PhyInterface : 1;							// @3
			volatile enum UMode {
				ULPI,
				UTMI,
			}  ModeSelect : 1;										// @4
			volatile bool fsintf : 1;								// @5
			volatile bool physel : 1;								// @6
			volatile bool ddrsel : 1;								// @7
			volatile bool SrpCapable : 1;							// @8
			volatile bool HnpCapable : 1;							// @9
			volatile unsigned usbtrdtim : 4;						// @10
			volatile unsigned reserved1 : 1;						// @14
			volatile bool phy_lpm_clk_sel : 1;						// @15
			volatile bool otgutmifssel : 1;							// @16
			volatile bool UlpiFsls : 1;								// @17
			volatile bool ulpi_auto_res : 1;						// @18
			volatile bool ulpi_clk_sus_m : 1;						// @19
			volatile bool UlpiDriveExternalVbus : 1;				// @20
			volatile bool ulpi_int_vbus_indicator : 1;				// @21
			volatile bool TsDlinePulseEnable : 1;					// @22
			volatile bool indicator_complement : 1;					// @23
			volatile bool indicator_pass_through : 1;				// @24
			volatile bool ulpi_int_prot_dis : 1;					// @25
			volatile bool ic_usb_capable : 1;						// @26
			volatile bool ic_traffic_pull_remove : 1;				// @27
			volatile bool tx_end_delay : 1;							// @28
			volatile bool force_host_mode : 1;						// @29
			volatile bool force_dev_mode : 1;						// @30
			volatile unsigned _reserved31 : 1;						// @31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{							 USB CORE RESET STRUCTURE					    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) CoreReset {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile bool CoreSoft : 1;								// @0
			volatile bool HclkSoft : 1;								// @1
			volatile bool HostFrameCounter : 1;						// @2
			volatile bool InTokenQueueFlush : 1;					// @3
			volatile bool ReceiveFifoFlush : 1;						// @4
			volatile bool TransmitFifoFlush : 1;					// @5
			volatile unsigned TransmitFifoFlushNumber : 5;			// @6
			volatile unsigned _reserved11_29 : 19;					// @11
			volatile bool DmaRequestSignal : 1;						// @30
			volatile bool AhbMasterIdle : 1;						// @31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{	       INTERRUPT BITS ON THE USB CORE OF THE DESIGNWARE 2.0		        }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) CoreInterrupts {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile bool CurrentMode : 1;							// @0
			volatile bool ModeMismatch : 1;							// @1
			volatile bool Otg : 1;									// @2
			volatile bool DmaStartOfFrame : 1;						// @3
			volatile bool ReceiveStatusLevel : 1;					// @4
			volatile bool NpTransmitFifoEmpty : 1;					// @5
			volatile bool ginnakeff : 1;							// @6
			volatile bool goutnakeff : 1;							// @7
			volatile bool ulpick : 1;								// @8
			volatile bool I2c : 1;									// @9
			volatile bool EarlySuspend : 1;							// @10
			volatile bool UsbSuspend : 1;							// @11
			volatile bool UsbReset : 1;								// @12
			volatile bool EnumerationDone : 1;						// @13
			volatile bool IsochronousOutDrop : 1;					// @14
			volatile bool eopframe : 1;								// @15
			volatile bool RestoreDone : 1;							// @16
			volatile bool EndPointMismatch : 1;						// @17
			volatile bool InEndPoint : 1;							// @18
			volatile bool OutEndPoint : 1;							// @19
			volatile bool IncompleteIsochronousIn : 1;				// @20
			volatile bool IncompleteIsochronousOut : 1;				// @21
			volatile bool fetsetup : 1;								// @22
			volatile bool ResetDetect : 1;							// @23
			volatile bool Port : 1;									// @24
			volatile bool HostChannel : 1;							// @25
			volatile bool HpTransmitFifoEmpty : 1;					// @26
			volatile bool LowPowerModeTransmitReceived : 1;			// @27
			volatile bool ConnectionIdStatusChange : 1;				// @28
			volatile bool Disconnect : 1;							// @29
			volatile bool SessionRequest : 1;						// @30
			volatile bool Wakeup : 1;								// @31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{				 USB CORE NON PERIODIC FIFO STATUS STRUCTURE			    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) NonPeriodicFifoStatus {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile unsigned SpaceAvailable : 16;					// @0
			volatile unsigned QueueSpaceAvailable : 8;				// @16
			volatile unsigned Terminate : 1;						// @24
			volatile enum {
				InOut = 0,
				ZeroLengthOut = 1,
				PingCompleteSplit = 2,
				ChannelHalt = 3,
			} TokenType : 2;										// @25
			volatile unsigned Channel : 4;							// @27
			volatile unsigned Odd : 1;								// @31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{				 USB CORE NON PERIODIC INFO STRUCTURE					    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) CoreNonPeriodicInfo {
	volatile __attribute__((aligned(4))) struct FifoSize Size;		// +0x28
	volatile __attribute__((aligned(4))) const struct NonPeriodicFifoStatus Status;	// Read Only +0x2c
};

/*--------------------------------------------------------------------------}
{						 USB CORE HARDWARE STRUCTURE					    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) CoreHardware {
	struct __attribute__((__packed__, aligned(1))) {
		volatile const unsigned Direction0 : 2;						// @0
		volatile const unsigned Direction1 : 2;						// @2
		volatile const unsigned Direction2 : 2;						// @4
		volatile const unsigned Direction3 : 2;						// @6
		volatile const unsigned Direction4 : 2;						// @8
		volatile const unsigned Direction5 : 2;						// @10
		volatile const unsigned Direction6 : 2;						// @12
		volatile const unsigned Direction7 : 2;						// @14
		volatile const unsigned Direction8 : 2;						// @16
		volatile const unsigned Direction9 : 2;						// @18
		volatile const unsigned Direction10 : 2;					// @20
		volatile const unsigned Direction11 : 2;					// @22
		volatile const unsigned Direction12 : 2;					// @24
		volatile const unsigned Direction13 : 2;					// @26
		volatile const unsigned Direction14 : 2;					// @28
		volatile const unsigned Direction15 : 2;					// @30
		volatile const enum {
			HNP_SRP_CAPABLE,
			SRP_ONLY_CAPABLE,
			NO_HNP_SRP_CAPABLE,
			SRP_CAPABLE_DEVICE,
			NO_SRP_CAPABLE_DEVICE,
			SRP_CAPABLE_HOST,
			NO_SRP_CAPABLE_HOST,
		} OperatingMode : 3;										// @32-34
		volatile const enum {
			SlaveOnly,
			ExternalDma,
			InternalDma,
		} Architecture : 2;											// @35
		volatile bool PointToPoint : 1;								// @37
		volatile const enum {
			NotSupported,
			Utmi,
			Ulpi,
			UtmiUlpi,
		} HighSpeedPhysical : 2;									// @38-39
		volatile const enum {
			Physical0,
			Dedicated,
			Physical2,
			Physcial3,
		} FullSpeedPhysical : 2;									// @40-41
		volatile const unsigned DeviceEndPointCount : 4;			// @42
		volatile const unsigned HostChannelCount : 4;				// @46
		volatile const bool SupportsPeriodicEndpoints : 1;			// @50
		volatile const bool DynamicFifo : 1;						// @51
		volatile const bool multi_proc_int : 1;						// @52
		volatile const unsigned _reserver21 : 1;					// @53
		volatile const unsigned NonPeriodicQueueDepth : 2;			// @54
		volatile const unsigned HostPeriodicQueueDepth : 2;			// @56
		volatile const unsigned DeviceTokenQueueDepth : 5;			// @58
		volatile const bool EnableIcUsb : 1;						// @63
		volatile const unsigned TransferSizeControlWidth : 4;		// @64
		volatile const unsigned PacketSizeControlWidth : 3;			// @68
		volatile const bool otg_func : 1;							// @71
		volatile const bool I2c : 1;								// @72
		volatile const bool VendorControlInterface : 1;				// @73
		volatile const bool OptionalFeatures : 1;					// @74
		volatile const bool SynchronousResetType : 1;				// @75
		volatile const bool AdpSupport : 1;							// @76
		volatile const bool otg_enable_hsic : 1;					// @77
		volatile const bool bc_support : 1;							// @78
		volatile const bool LowPowerModeEnabled : 1;				// @79
		volatile const unsigned FifoDepth : 16;						// @80
		volatile const unsigned PeriodicInEndpointCount : 4;		// @96
		volatile const bool PowerOptimisation : 1;					// @100
		volatile const bool MinimumAhbFrequency : 1;				// @101
		volatile const bool PartialPowerOff : 1;					// @102
		volatile const unsigned _reserved103_109 : 7;				// @103
		volatile const enum {
			Width8bit,
			Width16bit,
			Width8or16bit,
		} UtmiPhysicalDataWidth : 2;								// @110
		volatile const unsigned ModeControlEndpointCount : 4;		// @112
		volatile const bool ValidFilterIddigEnabled : 1;			// @116
		volatile const bool VbusValidFilterEnabled : 1;				// @117
		volatile const bool ValidFilterAEnabled : 1;				// @118
		volatile const bool ValidFilterBEnabled : 1;				// @119
		volatile const bool SessionEndFilterEnabled : 1;			// @120
		volatile const bool ded_fifo_en : 1;						// @121
		volatile const unsigned InEndpointCount : 4;				// @122
		volatile const bool DmaDescription : 1;						// @126
		volatile const bool DmaDynamicDescription : 1;				// @127
	};
};

/*--------------------------------------------------------------------------}
{                       USB CORE PERIODIC INFO STRUCTURE				    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) CorePeriodicInfo {
	volatile __attribute__((aligned(4))) struct FifoSize HostSize;	// +0x100
	volatile __attribute__((aligned(4))) struct FifoSize DataSize[15];// +0x104
};

/***************************************************************************}
{         PRIVATE INTERNAL DESIGNWARE 2.0 HOST REGISTER STRUCTURES          }
****************************************************************************/

enum ClockRate {
	Clock30_60MHz,													// 30-60Mhz clock to USB
	Clock48MHz,														// 48Mhz clock to USB
	Clock6MHz,														// 6Mhz clock to USB
};

/*--------------------------------------------------------------------------}
{                          USB HOST CONFIG STRUCTURE					    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) HostConfig {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile unsigned ClockRate : 2;						// @0
			volatile bool FslsOnly : 1;								// @2
			volatile unsigned _reserved3_6 : 4;						// @3
			volatile unsigned en_32khz_susp : 1;					// @7
			volatile unsigned res_val_period : 8;					// @8
			volatile unsigned _reserved16_22 : 7;					// @16
			volatile bool EnableDmaDescriptor : 1;					// @23
			volatile unsigned FrameListEntries : 2;					// @24
			volatile bool PeriodicScheduleEnable : 1;				// @26
			volatile bool PeriodicScheduleStatus : 1;				// @27
			volatile unsigned reserved28_30 : 3;					// @28
			volatile bool mode_chg_time : 1;						// @31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{                       USB HOST FRAME INTERVAL STRUCTURE				    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) HostFrameInterval {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile unsigned Interval : 16;						// @0
			volatile bool DynamicFrameReload : 1;					// @16
			volatile unsigned _reserved17_31 : 15;					// @17-31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{                       USB HOST FRAME CONTROL STRUCTURE				    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) HostFrameControl {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile unsigned FrameNumber : 16;						// @0
			volatile unsigned FrameRemaining : 16;					// @16
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{                         USB FIFO STATUS STRUCTURE						    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) HostFifoStatus {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile unsigned SpaceAvailable : 16;					// @0
			volatile unsigned QueueSpaceAvailable : 8;				// @16
			volatile unsigned Terminate : 1;						// @24
			volatile enum {
				ZeroLength = 0,
				Ping = 1,
				Disable = 2,
			} TokenType : 2;										// @25
			volatile unsigned Channel : 4;							// @27
			volatile unsigned Odd : 1;								// @31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{                         USB HOST PORT STRUCTURE						    }
{--------------------------------------------------------------------------*/
/* Due to the inconsistent design of the bits in this register, sometime it requires  zeroing 
   bits in the register before the write, so you do not unintentionally write 1's to them. */
#define HOSTPORTMASK  ~0x2E								// These are the funky bits on this register and we "NOT" them to make "AND" mask
struct __attribute__((__packed__, aligned(4))) HostPort {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile bool Connect : 1;								// @0
			volatile bool ConnectChanged : 1;						// @1
			volatile bool Enable : 1;								// @2
			volatile bool EnableChanged : 1;						// @3
			volatile bool OverCurrent : 1;							// @4
			volatile bool OverCurrentChanged : 1;					// @5
			volatile bool Resume : 1;								// @6
			volatile bool Suspend : 1;								// @7
			volatile bool Reset : 1;								// @8
			volatile unsigned _reserved9 : 1;						// @9
			volatile unsigned PortLineStatus : 2;					// @10
			volatile bool Power : 1;								// @12
			volatile unsigned TestControl : 4;						// @13
			volatile UsbSpeed Speed : 2;							// @17
			volatile unsigned _reserved19_31 : 13;					// @19-31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{                USB HOST CHANNEL CHARACTERISTIC STRUCTURE				    }
{--------------------------------------------------------------------------*/
struct HostChannelCharacteristic {
	union {
		struct {
			unsigned max_packet_size : 11;					// @0-10	Maximum packet size the endpoint is capable of sending or receiving
			unsigned endpoint_number : 4;					// @11-14	Endpoint number (low 4 bits of bEndpointAddress)
			unsigned endpoint_direction : 1;				// @15		Endpoint direction 1=IN, 0=OUT
			unsigned _reserved : 1;							// @16
			unsigned low_speed : 1;							// @17		1 when the device being communicated with is at low speed, 0 otherwise
			unsigned endpoint_type : 2;						// @18-19	Endpoint type (low 2 bits of bmAttributes)
			unsigned packets_per_frame : 2;					// @20-21	Maximum number of transactions that can be executed per microframe
			unsigned device_address : 7;					// @22-28	USB device address of the device on which the endpoint is located
			unsigned odd_frame : 1;							// @29		Before enabling channel must be set to opposite of low bit of host_frame_number
			unsigned channel_disable : 1;					// @30		Software can set this to 1 to halt the channel
			unsigned channel_enable : 1;					// @31		Software can set this to 1 to enable the channel
		} __packed;
		volatile uint32_t Raw32;							// Union to access all 32 bits as a uint32_t
	};
} __packed;

/*--------------------------------------------------------------------------}
{                USB HOST CHANNEL SPLIT CONTROL STRUCTURE				    }
{--------------------------------------------------------------------------*/
struct HostChannelSplitControl {
	union {
		struct {
			unsigned port_address : 7;						// @0-6		0-based index of the port on the high-speed hub Transaction Translator occurs
			unsigned hub_address : 7;						// @7-13	USB device address of the high-speed hub that acts as Transaction Translator
			unsigned transaction_position : 2;				// @14-15	If we are processing split the transation position Begin=2,End=1,Middle=0,All=3
			unsigned complete_split : 1;					// @16		1 to complete a Split transaction, 0 = normal transaction
			unsigned _reserved : 14;						// @17-30
			unsigned split_enable : 1;						// @31		Set to 1 to enable Split Transactions
		} __packed;
		volatile uint32_t Raw32;							// Union to access all 32 bits as a uint32_t
	};
} __packed;

/*--------------------------------------------------------------------------}
{                USB HOST CHANNEL TRANSFER SIZE STRUCTURE				    }
{--------------------------------------------------------------------------*/
struct HostTransferSize {
	union {
		struct {
			unsigned size : 19;								// @0-18	Size of data to send or receive, in bytes and can be greater than maximum packet length
			unsigned packet_count : 10;						// @19-28   Number of packets left to transmit or maximum number of packets left to receive
			enum PacketId {
				USB_PID_DATA0 = 0,
				USB_PID_DATA1 = 2,
				USB_PID_DATA2 = 1,
				USB_PID_SETUP = 3,
				USB_MDATA = 3,
			} packet_id : 2;								// @29		Various packet phase ID
			unsigned do_ping : 1;							// @31		
		} __packed;
		volatile uint32_t Raw32;							// Union to access all 32 bits as a uint32_t
	};
} __packed;

/*--------------------------------------------------------------------------}
{					  USB HOST CHANNEL STRUCTURE						    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) HostChannel {
	volatile __attribute__((aligned(4))) struct HostChannelCharacteristic Characteristic;	// +0x0
	volatile __attribute__((aligned(4))) struct HostChannelSplitControl SplitCtrl;			// +0x4
	volatile __attribute__((aligned(4))) struct ChannelInterrupts Interrupt;				// +0x8
	volatile __attribute__((aligned(4))) struct ChannelInterrupts InterruptMask;			// +0xc
	volatile __attribute__((aligned(4))) struct HostTransferSize TransferSize;				// +0x10
	volatile __attribute__((aligned(4))) uint32_t  DmaAddr;									// +0x14
	volatile __attribute__((aligned(4))) uint32_t _reserved18;								// +0x18
	volatile __attribute__((aligned(4))) uint32_t _reserved1c;								// +0x1c
};

/*--------------------------------------------------------------------------}
{					DWC POWER AND CLOCK REGISTER STRUCTURE				    }
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) PowerReg {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			volatile bool StopPClock : 1;							// @0
			volatile bool GateHClock : 1;							// @1
			volatile bool PowerClamp : 1;							// @2
			volatile bool PowerDownModules : 1;						// @3
			volatile bool PhySuspended : 1;							// @4
			volatile bool EnableSleepClockGating : 1;				// @5
			volatile bool PhySleeping : 1;							// @6
			volatile bool DeepSleep : 1;							// @7
			volatile unsigned _reserved8_31 : 24;					// @8-31
		};
		volatile uint32_t Raw32;									// Union to access all 32 bits as a uint32_t
	};
};

/*--------------------------------------------------------------------------}
{ 				USB control used solely by internal routines				}
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__, aligned(4))) UsbSendControl {
	union {
		struct __attribute__((__packed__, aligned(1))) {
			unsigned SplitTries : 8;								// @0  Count of attempts to send packet as a split
			unsigned PacketTries : 8;								// @8  Count of attempts to send current packet
			unsigned GlobalTries : 8;								// @16 Count of global tries (more serious errors increment)
			unsigned reserved : 3;									// @24 Padding to make 32 bit
			bool	 LongerDelay : 1;								// @27 Longer delay .. not yet was response
			bool	 ActionResendSplit : 1;							// @28 Resend split packet
			bool	 ActionRetry : 1;								// @29 Retry sending 
			bool	 ActionFatalError : 1;							// @30 Some fatal error occured ... so bail
			bool	 Success : 1;									// @31 Success .. tansfer complete
		};
		uint32_t Raw32;												// Union to access all 32 bits as a uint32_t
	};
};

/***************************************************************************}
{    PRIVATE POINTERS TO ALL OUR DESIGNWARE 2.0 HOST REGISTER STRUCTURES    }
****************************************************************************/

#define USB_CORE_OFFSET  0x980000	// USB CORE OFFSET FROM PERIPHERAL IO BASE ADDRESS

/*--------------------------------------------------------------------------}
{					 DWC USB CORE REGISTER POINTERS						    }
{--------------------------------------------------------------------------*/
#define DWC_CORE_OTGCONTROL			((volatile __attribute__((aligned(4))) struct CoreOtgControl*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x00))
#define DWC_CORE_OTGINTERRUPT		((volatile __attribute__((aligned(4))) struct CoreOtgInterrupt*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x04))
#define DWC_CORE_AHB				((volatile __attribute__((aligned(4))) struct CoreAhb*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x08))
#define DWC_CORE_CONTROL			((volatile __attribute__((aligned(4))) struct UsbControl*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x0C))
#define DWC_CORE_RESET				((volatile __attribute__((aligned(4))) struct CoreReset*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x10))
#define DWC_CORE_INTERRUPT			((volatile __attribute__((aligned(4))) struct CoreInterrupts*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x14))
#define DWC_CORE_INTERRUPTMASK		((volatile __attribute__((aligned(4))) struct CoreInterrupts*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x18))
#define DWC_CORE_RECEIVESIZE		((volatile __attribute__((aligned(4))) uint32_t*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x24))
#define DWC_CORE_NONPERIODICFIFO	((volatile __attribute__((aligned(4))) struct CoreNonPeriodicInfo*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x28))
#define DWC_CORE_USERID				((volatile __attribute__((aligned(4))) uint32_t*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x3C))
#define DWC_CORE_VENDORID			((volatile __attribute__((aligned(4))) const uint32_t*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x40))
#define DWC_CORE_HARDWARE			((volatile __attribute__((aligned(4))) const struct CoreHardware*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x44))
#define DWC_CORE_PERIODICINFO		((volatile __attribute__((aligned(4))) struct CorePeriodicInfo*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x100))

/*--------------------------------------------------------------------------}
{					DWC USB HOST REGISTER POINTERS						    }
{--------------------------------------------------------------------------*/
#define DWC_HOST_CONFIG				((volatile __attribute__((aligned(4))) struct HostConfig*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x400))
#define DWC_HOST_FRAMEINTERVAL		((volatile __attribute__((aligned(4))) struct HostFrameInterval*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x404))
#define DWC_HOST_FRAMECONTROL		((volatile __attribute__((aligned(4))) struct HostFrameControl*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x408))
#define DWC_HOST_FIFOSTATUS			((volatile __attribute__((aligned(4))) struct HostFifoStatus*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x410))
#define DWC_HOST_INTERRUPT			((volatile __attribute__((aligned(4))) uint32_t*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x414))
#define DWC_HOST_INTERRUPTMASK		((volatile __attribute__((aligned(4))) uint32_t*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x418))
#define DWC_HOST_FRAMELIST			((volatile __attribute__((aligned(4))) uint32_t*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x41C))
#define DWC_HOST_PORT				((volatile __attribute__((aligned(4))) struct HostPort*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x440))
#define DWC_HOST_CHANNEL			((volatile __attribute__((aligned(4))) struct HostChannel*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0x500))

/*--------------------------------------------------------------------------}
{					DWC POWER AND CLOCK REGISTER POINTER				    }
{--------------------------------------------------------------------------*/
#define DWC_POWER_AND_CLOCK			((__attribute__((aligned(4))) struct PowerReg*)(uintptr_t)(RPi_IO_Base_Addr + USB_CORE_OFFSET + 0xE00))


/*--------------------------------------------------------------------------}
{				 INTERNAL USB STRUCTURE COMPILE TIME CHECKS		            }
{--------------------------------------------------------------------------*/
/* GIVEN THE AMOUNT OF PRECISE PACKING OF THESE STRUCTURES .. IT'S PRUDENT */
/* TO CHECK THEM AT COMPILE TIME. USE IS POINTLESS IF THE SIZES ARE WRONG. */
/*-------------------------------------------------------------------------*/
/* If you have never seen compile time assertions it's worth google search */
/* on "Compile Time Assertions". It is part of the C11++ specification and */
/* all compilers that support the standard will have them (GCC, MSC inc)   */
/*-------------------------------------------------------------------------*/
#include <assert.h>								// Need for compile time static_assert

/* DESIGNWARE 2.0 REGISTERS */
static_assert(sizeof(struct CoreOtgControl) == 0x04, "Register/Structure should be 32bits (4 bytes)");
static_assert(sizeof(struct CoreOtgInterrupt) == 0x04, "Register/Structure should be 32bits (4 bytes)");
static_assert(sizeof(struct CoreAhb) == 0x04, "Register/Structure should be 32bits (4 bytes)");
static_assert(sizeof(struct UsbControl) == 0x04, "Register/Structure should be 32bits (4 bytes)");
static_assert(sizeof(struct CoreReset) == 0x04, "Register/Structure should be 32bits (4 bytes)");
static_assert(sizeof(struct CoreInterrupts) == 0x04, "Register/Structure should be 32bits (4 bytes)");

static_assert(sizeof(struct CoreNonPeriodicInfo) == 0x08, "Register/Structure should be 2x32bits (8 bytes)");

static_assert(sizeof(struct CoreHardware) == 0x10, "Register/Structure should be 4x32bits (16 bytes)");

static_assert(sizeof(struct HostChannel) == 0x20, "Register/Structure should be 8x32bits (32 bytes)");

/* USB SPECIFICATION STRUCTURES */
static_assert(sizeof(struct HubPortFullStatus) == 0x04, "Structure should be 32bits (4 bytes)");
static_assert(sizeof(struct HubFullStatus) == 0x04, "Structure should be 32bits (4 bytes)");
static_assert(sizeof(struct UsbDescriptorHeader) == 0x02, "Structure should be 2 bytes");
static_assert(sizeof(struct UsbEndpointDescriptor) == 0x07, "Structure should be 7 bytes");
static_assert(sizeof(struct UsbDeviceRequest) == 0x08, "Structure should be 8 bytes");
static_assert(sizeof(struct HubDescriptor) == 0x09, "Structure should be 9 bytes");
static_assert(sizeof(struct UsbInterfaceDescriptor) == 0x09, "Structure should be 9 bytes");
static_assert(sizeof(struct usb_configuration_descriptor) == 0x09, "Structure should be 9 bytes");
static_assert(sizeof(struct usb_device_descriptor) == 0x12, "Structure should be 18 bytes");

/* INTERNAL STRUCTURES */
static_assert(sizeof(struct UsbSendControl) == 0x04, "Structure should be 32bits (4 bytes)");

/***************************************************************************}
{					      PRIVATE INTERNAL CONSTANTS	                    }
****************************************************************************/

/**
 * Number of DWC host channels, each of which can be used for an independent
 * USB transfer.  On the BCM2835 (Raspberry Pi), 8 are available.  This is
 * documented on page 201 of the BCM2835 ARM Peripherals document.
 */
#define DWC_NUM_CHANNELS 8

/**
 * Maximum packet size of any USB endpoint.  1024 is the maximum allowed by USB
 * 2.0.  Most endpoints will provide maximum packet sizes much smaller than
 * this.
 */
#define USB2_MAX_PACKET_SIZE 1024

/***************************************************************************}
{					      PRIVATE INTERNAL VARIABLES	                    }
****************************************************************************/

/* Aligned buffers for DMA which need to also be multiple of 4 bytes */
/* Fortunately max packet size under USB2 is 1024 so that is a given */
static uint8_t aligned_bufs[DWC_NUM_CHANNELS][USB2_MAX_PACKET_SIZE] __aligned(4);


bool PhyInitialised = false;
uint8_t RootHubDeviceNumber = 0;

struct UsbDevice DeviceTable[MaximumDevices] = { 0 };				// Usb node device allocation table
#define MaximumHubs	16												// Maximum number of HUB payloads we will allow
struct HubDevice HubTable[MaximumHubs] = { 0 };						// Usb hub device allocation table
#define MaximumHids 16												// Maximum number of HID payloads we will allow
struct HidDevice HidTable[MaximumHids] = { 0 };						// Usb hid device allocation table


/***************************************************************************}
{                PRIVATE INTERNAL CONSTANT DEFINITIONS                      }
****************************************************************************/
/*--------------------------------------------------------------------------}
{			USB2.0 DEVICE DESCRIPTOR BLOCK FOR OUR "FAKED" ROOTHUB 			}
{--------------------------------------------------------------------------*/
struct __attribute__((aligned(4))) usb_device_descriptor RootHubDeviceDescriptor = {
	.bLength = sizeof(struct usb_device_descriptor),
	.bDescriptorType = USB_DESCRIPTOR_TYPE_DEVICE,
	.bcdUSB = 0x0200,
	.bDeviceClass = DeviceClassHub,
	.bDeviceSubClass = 0,
	.bDeviceProtocol = 0,
	.bMaxPacketSize0 = 64,
	.idVendor = 0,
	.idProduct = 0,
	.bcdDevice = 0x0100,
	.iManufacturer = 0,
	.iProduct = 1,									// String 1 see below .. says "FAKED Root Hub (tm)"
	.iSerialNumber = 0,
	.bNumConfigurations = 1,
};

/*--------------------------------------------------------------------------}
{  Hard-coded configuration descriptor, along with an associated interface  }
{  descriptor and endpoint descriptor, for the "faked" root hub.			}
{--------------------------------------------------------------------------*/
struct __attribute__((__packed__)) RootHubConfig {
	struct usb_configuration_descriptor Configuration;
	struct UsbInterfaceDescriptor Interface;
	struct UsbEndpointDescriptor Endpoint;
};

struct __attribute__((aligned(4))) RootHubConfig root_hub_configuration = {
	.Configuration = {
		.bLength = sizeof(struct usb_configuration_descriptor),
		.bDescriptorType = USB_DESCRIPTOR_TYPE_CONFIGURATION,
		.wTotalLength = sizeof(root_hub_configuration),
		.bNumInterfaces = 1,
		.bConfigurationValue = 1,
		.iConfiguration = 2,
		.RemoteWakeup = false,
		.SelfPowered = true,
		._reserved7 = 1,
		.bMaxPower = 0,
	},
	.Interface = {
		.Header = {
			.DescriptorLength = sizeof(struct UsbInterfaceDescriptor),
			.DescriptorType = USB_DESCRIPTOR_TYPE_INTERFACE,
		},
		.Number = 0,
		.AlternateSetting = 0,
		.EndpointCount = 1,
		.Class = InterfaceClassHub,
		.SubClass = 0,
		.Protocol = 0,
		.StringIndex = 0,
	},
	.Endpoint = {
		.Header = {
			.DescriptorLength = sizeof(struct UsbEndpointDescriptor),
			.DescriptorType = USB_DESCRIPTOR_TYPE_ENDPOINT,
		},
		.EndpointAddress = {
			.Number = 1,
			.Direction = USB_DIRECTION_IN,
		},
		.Attributes = {
			.Type = USB_TRANSFER_TYPE_INTERRUPT,
		},
		.Packet = {
			.MaxSize = 64,
		},
		.Interval = 0xff,
	},
};

/*--------------------------------------------------------------------------}
{		  USB2.0 DESCRIPTION STRING0 FOR OUR "FAKED" ROOTHUB 				}
{--------------------------------------------------------------------------*/
struct __attribute__((aligned(4))) UsbStringDescriptor RootHubString0 = {
	.Header = {
		.DescriptorLength = 4,
		.DescriptorType = USB_DESCRIPTOR_TYPE_STRING,
	},
	.Data = {
		0x0409,
	},
};

#define RootHubString u"FAKED Root Hub (tm)"				// UTF string
/*--------------------------------------------------------------------------}
{		  USB2.0 DESCRIPTION STRING1 FOR OUR "FAKED" ROOTHUB				}
{--------------------------------------------------------------------------*/
struct __attribute__((aligned(4))) UsbStringDescriptor RootHubString1 = {
	.Header = {
		.DescriptorLength = sizeof(RootHubString) + 2,
		.DescriptorType = USB_DESCRIPTOR_TYPE_STRING,
	},
	.Data = {
		RootHubString,
	},
};

#define RootHubConfigString u"FAKE config string"				// UTF string
/*--------------------------------------------------------------------------}
{		  USB2.0 DESCRIPTION STRING3 FOR OUR "FAKED" ROOTHUB				}
{--------------------------------------------------------------------------*/
struct __attribute__((aligned(4))) UsbStringDescriptor RootHubString2 = {
	.Header = {
		.DescriptorLength = sizeof(RootHubConfigString) + 2,
		.DescriptorType = USB_DESCRIPTOR_TYPE_STRING,
	},
	.Data = {
		RootHubConfigString,
	},
};

/*--------------------------------------------------------------------------}
{			USB2.0 HUB DESCRIPTION FOR OUR "FAKED" ROOTHUB 					}
{--------------------------------------------------------------------------*/
struct __attribute__((aligned(4))) HubDescriptor RootHubDescriptor = {
	.Header = {
		.DescriptorLength = sizeof(struct HubDescriptor),
		.DescriptorType = USB_DESCRIPTOR_TYPE_HUB,
	},
	.PortCount = 1,
	.Attributes = {
		.PowerSwitchingMode = Global,
		.Compound = false,
		.OverCurrentProtection = Global,
		.ThinkTime = 0,
		.Indicators = false,
	},
	.PowerGoodDelay = 0,
	.MaximumHubPower = 0,
	.DeviceRemovable = { .Port1 = true },
	.PortPowerCtrlMask = 0xff,
};

/***************************************************************************}
{						 PRIVATE INTERNAL VARIABLES			                }
****************************************************************************/

/** Bitmap of channel free (1) or in-use (0) statuses.  */
static uint32_t chfree = 0;

/***************************************************************************}
{						 PRIVATE INTERNAL FUNCTIONS						    }
****************************************************************************/

/*-[INTERNAL: first_set_bit ]------------------------------------------------
. Find index of first set bit in a nonzero uint32_t
.--------------------------------------------------------------------------*/
static inline unsigned int first_set_bit (uint32_t word)
{
	return (31 - __builtin_clz(word));								// Return index of first set bit
}

/*-[INTERNAL: dwc_get_free_channel ]-----------------------------------------
. Finds and reserves an unused DWC USB host channel. This is blocking and
. will wait until a channel is available if all in use.
. RETURN: Index of the free channel
.--------------------------------------------------------------------------*/
static unsigned int dwc_get_free_channel(void)
{
	unsigned int chan;
	//wait(chfree_sema);												// Wait for a free channel	
	//ENTER_KERNEL_CRITICAL_SECTION();								// Must disable scheduler as we play with the free channels
	chan = first_set_bit(chfree);									// Find the first free channel .. there must be one because of semaphore
	chfree &= ~((uint32_t)1 << chan);								// Mark the channel as no longer free										
	//EXIT_KERNEL_CRITICAL_SECTION();									// Exit the critical section
	return chan;													// Return the channel
}

/*-[INTERNAL: dwc_release_channel ]-----------------------------------------
. Releases the given DWC USB host channel that was in use and marks as free.
.--------------------------------------------------------------------------*/
static void dwc_release_channel(unsigned int chan)
{
	//ENTER_KERNEL_CRITICAL_SECTION();								// Entering a critical section
	chfree |= ((uint32_t)1 << chan);								// Mark channel as free
	//EXIT_KERNEL_CRITICAL_SECTION();									// Exit the critical section
	//signal(chfree_sema);											// Signal channel free
}

/*==========================================================================}
{	 MY MEMORY COPY .. YEAH I AM OVER THE ARM MEMCOPY ALIGNMENT	ISSUES	    }
{==========================================================================*/
void myMemCopy (uint8_t* dest, uint8_t* source, uint32_t size){
	while (size) {													// While data to copy
		*dest++ = *source++;										// Copy 1 byte from source to dest and increment pointers
		size--;														// Decerement size
	}
}

/*==========================================================================}
{			    INTERNAL FAKE ROOT HUB MESSAGE HANDLER					    }
{==========================================================================*/
RESULT HcdProcessRootHubMessage (uint8_t* buffer, uint32_t bufferLength, struct UsbDeviceRequest *request, uint32_t *bytesTransferred)
{
	RESULT result = OK;
	uint32_t replyLength = 0;
	struct HostPort tempPort;
	union {										// Place a union over these to stop having to mess around .. its a 4 bytes whatever the case .. look carefully
		uint8_t* replyBytes;					// Pointer to bytes to return can be anything 
		uint8_t	reply8;							// 8 bit return
		uint16_t reply16;						// 16 bit return
		uint32_t reply32;						// 32 bit return
		struct HubFullStatus replyHub;			// Hub status return
		struct HubPortFullStatus replyPort;		// Port status return
	} replyBuf;
	bool ptrTransfer = false;					// Default is not a pointer transfer
	switch (request->Request) {
	/* Details on GetStatus from here http://www.beyondlogic.org/usbnutshell/usb6.shtml */
	case GetStatus:
		switch (request->Type) {
		case bmREQ_DEVICE_STATUS  /*0x80*/: 						// Device status request .. returns a 16 bit device status
			replyBuf.reply16 = 1;									// Only two bits in D0 = Self Powered, D1 = Remote Wakeup .. So 1 just self powered 
			replyLength = 2;										// Two byte response
			break;
		case bmREQ_INTERFACE_STATUS /* 0x81 */:						// Interface status request .. returns a 16 bit status
			replyBuf.reply16 = 0;									// Spec says two bytes of 0x00, 0x00. (Both bytes are reserved for future use)
			replyLength = 2;										// Two byte response
		case bmREQ_ENDPOINT_STATUS /* 0x82 */:						// Endpoint status request .. return a 16 bit status 
			replyBuf.reply16 = 0;									// Two bytes indicating the status (Halted/Stalled) of a endpoint. D0 = Stall .. 0 No stall for us 
			replyLength = 2;										// Two byte response
			break;
		case bmREQ_HUB_STATUS  /*0xa0*/:							// We are a hub class so we need a standard hub class get status return
			replyBuf.replyHub.Raw32 = 0;							// Zero all the status bits
			replyBuf.replyHub.Status.LocalPower = true;				// So we will return a HubFullStatus ... Just set LocalPower bit
			replyLength = 4;										// 4 bytes in size .. remember we checked all that in static asserts
			break;
		case bmREQ_PORT_STATUS /* 0xa3 */:							// PORT request .. Remember we have 1 port which is the actual physical hardware
			if (request->Index == 1) {								// Remember we have only one port so any other port is an error
				tempPort = *DWC_HOST_PORT;							// Read the host port
				replyBuf.replyPort.Raw32 = 0;						// Zero all the status bits
				replyBuf.replyPort.Status.Connected = tempPort.Connect;	// Transfer connect state
				replyBuf.replyPort.Status.Enabled = tempPort.Enable;// Transfer enabled state
				replyBuf.replyPort.Status.Suspended = tempPort.Suspend;	// Transfer suspend state
				replyBuf.replyPort.Status.OverCurrent = tempPort.OverCurrent;// Transfer overcurrent state
				replyBuf.replyPort.Status.Reset = tempPort.Reset;	// Transfer reset state
				replyBuf.replyPort.Status.Power = tempPort.Power;	// Transfer power state
				if (tempPort.Speed == USB_SPEED_HIGH)
					replyBuf.replyPort.Status.HighSpeedAttatched = true;// Set high speed state
				else if (tempPort.Speed == USB_SPEED_LOW)
					replyBuf.replyPort.Status.LowSpeedAttatched = true;	// Set low speed state
				replyBuf.replyPort.Status.TestMode = tempPort.TestControl;// Transfer test mode state
				replyBuf.replyPort.Change.ConnectedChanged = tempPort.ConnectChanged;// Transfer Connect changed state
				replyBuf.replyPort.Change.EnabledChanged = false;	// Always send back as zero .. dorky DWC2.0 doesn't have you have to monitor
				replyBuf.replyPort.Change.OverCurrentChanged = tempPort.OverCurrentChanged;// Transfer overcurrent changed state
				replyBuf.replyPort.Change.ResetChanged = false;		// Always send back as zero .. dorky DWC2.0 doesn't have you have to monitor
				replyLength = 4;									// 4 bytes in size .. remember we checked all that in static asserts
			} else result = ErrorArgument;							// Any other port than number 1 means the arguments are garbage
			break;
		default:
			result = ErrorArgument;									// Unknown argument provided on request GetStatus
			break;
		};
		break;
	/* Details on ClearFeature from here http://www.beyondlogic.org/usbnutshell/usb6.shtml */
	case ClearFeature:
		replyLength = 0;
		switch (request->Type) {
		case bmREQ_INTERFACE_FEATURE /*0x01*/:						// Interface clear feature requet
			break;													// Current USB Specification Revision 2 specifies no interface features.
		case bmREQ_ENDPOINT_FEATURE /*0x02*/:						// Endpoint set feature request
			break;													// 16 bits only option is Halt on D0 which we dont support
		case bmREQ_HUB_FEATURE      /*0x20*/:						// Hub clear feature request
			break;													// Only options DEVICE_REMOTE_WAKEUP and TEST_MODE neither which we support
		case  bmREQ_PORT_FEATURE /*0x23*/:							// Port clear feature request
			if (request->Index == 1) {								// Remember we have only one port so any other port is an error
				switch ((enum HubPortFeature)request->Value) {		// Check what request to clear is
				case FeatureEnable:
					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.Enable = true;							// Set enable change bit ... This is one of those set bit to write bits (bit 2)
					*DWC_HOST_PORT = tempPort;						// Write the value back
					break;
				case FeatureSuspend:
					DWC_POWER_AND_CLOCK->Raw32 = 0;
					timer_wait(5000);
					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.Resume = true;							// Set the bit we want
					*DWC_HOST_PORT = tempPort;						// Write the value back
					timer_wait(100000);
					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.Suspend = false;						// Clear the bit we want
					tempPort.Resume = false;						// Clear the bit we want
					*DWC_HOST_PORT = tempPort;						// Write the value back
					break;
				case FeaturePower:
					LOG("Physical host power off\n");
					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.Power = false;							// Clear the bit we want
					*DWC_HOST_PORT = tempPort;						// Write the value back
					break;
				case FeatureConnectionChange:
					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.ConnectChanged = true;					// Set connect change bit ... This is one of those set bit to write bits (bit 1)
					*DWC_HOST_PORT = tempPort;						// Write the value back
					break;
				case FeatureEnableChange:
					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.EnableChanged = true;					// Set enable change bit ... This is one of those set bit to write bits (bit 3)
					*DWC_HOST_PORT = tempPort;						// Write the value back
					break;
				case FeatureOverCurrentChange:
					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.OverCurrentChanged = true;				// Set overcurrent change bit ... This is one of those set bit to write bits (bit 5)
					*DWC_HOST_PORT = tempPort;						// Write the value back
					break;
				default:
					break;											// Any other clear feature rtequest just ignore
				}
			} else result = ErrorArgument;							// Any other port than number 1 means the arguments are garbage
			break;
		default:
			result = ErrorArgument;									// If it's not a device/interface/classor endpoint ClearFeature message is garbage
			break;
		}
		break;
	/* Details on SetFeature from here http://www.beyondlogic.org/usbnutshell/usb6.shtml */
	case SetFeature:
		replyLength = 0;
		switch (request->Type) {
		case bmREQ_INTERFACE_FEATURE /*0x01*/:						// Interface set feature requet
			break;													// Current USB Specification Revision 2 specifies no interface features.
		case bmREQ_ENDPOINT_FEATURE /*0x02*/:						// Endpoint set feature request
			break;													// 16 bits only option is Halt on D0 which we dont support
		case bmREQ_HUB_FEATURE      /*0x20*/:						// Hub set feature request
			break;													// 16 bits only options DEVICE_REMOTE_WAKEUP and TEST_MODE neither which we support
		case bmREQ_PORT_FEATURE /* 0x23 */:							// Port set feature request
			if (request->Index == 1) {								// Remember we have only one port so any other port is an error
				struct PowerReg tempPower;
				switch ((enum HubPortFeature)request->Value) {
				case FeatureReset:			
					tempPower = *DWC_POWER_AND_CLOCK;				// read power and clock
					tempPower.EnableSleepClockGating = false;		// Turn off sleep clock gating if on
					tempPower.StopPClock = false;					// Turn off stop clock
					*DWC_POWER_AND_CLOCK = tempPower;				// Write back to register
					timer_wait(10000);								// Small delay
					DWC_POWER_AND_CLOCK->Raw32 = 0;					// Now clear everything

					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.Suspend = false;						// Clear the bit we want
					tempPort.Reset = true;							// Set bit we want
					tempPort.Power = true;							// Set the bit we want
					*DWC_HOST_PORT = tempPort;						// Write the value back
					timer_wait(60000);
					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.Reset = false;							// Clear bit we want
					*DWC_HOST_PORT = tempPort;						// Write the value back
					LOG_DEBUG("Reset physical port .. rootHub %i\n", RootHubDeviceNumber);
					break;
				case FeaturePower:
					LOG("Physical host power on\n");
					tempPort = *DWC_HOST_PORT;						// Read the host port
					tempPort.Raw32 &= HOSTPORTMASK;					// Cleave off all the triggers
					tempPort.Power = true;							// Set the bit we want
					*DWC_HOST_PORT = tempPort;						// Write the value back
					break;
				default:
					break;
				}
			} else result = ErrorArgument;							// Any other port than number 1 means the argument are garbage .. remember 1 port on this hub
			break;
		default:
			result = ErrorArgument;									// If it's not a device/interface/class or endpoint SetFeature message is garbage
			break;
		}
		break;
	case SetAddress:
		replyLength = 0;
		RootHubDeviceNumber = request->Value;						// Move the roothub to address requested .. should always be from zero to 1
		break;
	/* Details on GetDescriptor from here http://www.beyondlogic.org/usbnutshell/usb5.shtml#DeviceDescriptors */
	case GetDescriptor:
		replyLength = 0;											// Preset no return data length	
		switch (request->Type) {
		case bmREQ_GET_DEVICE_DESCRIPTOR /*0x80*/:					// Device descriptor request
			switch ((request->Value >> 8) & 0xff) {
			case USB_DESCRIPTOR_TYPE_DEVICE:
				replyLength = sizeof(RootHubDeviceDescriptor);		// Size of our fake hub descriptor
				replyBuf.replyBytes = (uint8_t*)&RootHubDeviceDescriptor;// Pointer to our fake roothub descriptor
				ptrTransfer = true;									// Set pointer transfer flag
				break;
			case USB_DESCRIPTOR_TYPE_CONFIGURATION:
				replyLength = sizeof(root_hub_configuration);		// Size of our fake config descriptor
				replyBuf.replyBytes = (uint8_t*)&root_hub_configuration;// Pointer to our fake roothub configuration
				ptrTransfer = true;									// Set pointer transfer flag
				break;
			case USB_DESCRIPTOR_TYPE_STRING:
				switch (request->Value & 0xff) {
				case 0x0:											
					replyLength = RootHubString0.Header.DescriptorLength;// Length of string decriptor 0
					replyBuf.replyBytes = (uint8_t*)&RootHubString0;// Pointer to string 0
					ptrTransfer = true;								// Set pointer transfer flag
					break;
				case 0x1:				
					replyLength = RootHubString1.Header.DescriptorLength;// Length of string descriptor 1
					replyBuf.replyBytes = (uint8_t*)&RootHubString1;// Pointer to string 1
					ptrTransfer = true;								// Set pointer transfer flag
					break;
				case 0x2:											// Return our fake roothub string2				
					replyLength = RootHubString2.Header.DescriptorLength;// Length of string descriptor 2
					replyBuf.replyBytes = (uint8_t*)&RootHubString2;// Pointer to string 2
					ptrTransfer = true;								// Set pointer transfer flag
					break;
				default:
					break;
				}
				break;
			default:
				result = ErrorArgument;								// Unknown get descriptor type
			}
			break;
		case  bmREQ_GET_HUB_DESCRIPTOR /*0xa0*/:					// RootHub descriptor requested
			replyLength = RootHubDescriptor.Header.DescriptorLength;// Length of our descriptor for our fake hub
			replyBuf.replyBytes = (uint8_t*)&RootHubDescriptor;		// Pointer to our fake roothu descriptor
			ptrTransfer = true;										// Set pointer transfer flag
			break;
		default:
			result = ErrorArgument;									// Besides HUB and DEVICE descriptors our fake hub doesn't know
			break;
		}
		break;
	case GetConfiguration:											// Get configuration message
		replyBuf.reply8 = 0x1;										// Return 1 rememeber we only have 1 config so can't be anything else
		replyLength = 1;											// Reply is a byte
		break;
	case SetConfiguration:											// Set configuration message
		replyLength = 0;											// Just ignore it we have 1 fixed config
		break;
	default:														// Any other message is unknown
		result = ErrorArgument;										// Return error with argument
		break;
	}
	if (replyLength > bufferLength) replyLength = bufferLength;		// The buffer length does not have enough room so truncate our respone to fit
	if (ptrTransfer) myMemCopy(&buffer[0], replyBuf.replyBytes, replyLength); // For a pointer transfer replyBuf has the pointer
	   else myMemCopy(&buffer[0], (uint8_t*)&replyBuf, replyLength);// Otherwise we want the raw data in the replyBuf
	if (bytesTransferred) *bytesTransferred = replyLength;			// If bytes transferred return requested provide it
	return result;													// Return result
}

/*==========================================================================}
{					   INTERNAL HOST CONTROL FUNCTIONS					    }
{==========================================================================*/

/*-INTERNAL: PowerOnUsb------------------------------------------------------
 Uses PI mailbox to turn power onto USB see website about command 0x28001
 https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT PowerOnUsb(void) {
	uint32_t __attribute__((aligned(16))) mailbox_message[8];
	mailbox_message[0] = sizeof(mailbox_message);
	mailbox_message[1] = 0;
	mailbox_message[2] = MAILBOX_TAG_SET_POWER_STATE;
	mailbox_message[3] = 8;
	mailbox_message[4] = 8;
	mailbox_message[5] = 0x3;    // device = USB
	mailbox_message[6] = 0x1;	 // 1 = on	
	mailbox_message[7] = 0x0;

	mailbox_write(MB_CHANNEL_TAGS, ARMaddrToGPUaddr(&mailbox_message[0]));
	mailbox_read(MB_CHANNEL_TAGS);

	if ((mailbox_message[1] == 0x80000000) && (mailbox_message[4] == 0x80000008)) {
		return OK;
	}
	return ErrorDevice;												// Failed to turn on
}

/*-INTERNAL: PowerOffUsb-----------------------------------------------------
 Uses PI mailbox to turn power onto USB see website about command 0x28001
 https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT PowerOffUsb(void) {
	uint32_t __attribute__((aligned(16))) mailbox_message[8];
	mailbox_message[0] = sizeof(mailbox_message);
	mailbox_message[1] = 0;
	mailbox_message[2] = MAILBOX_TAG_SET_POWER_STATE;
	mailbox_message[3] = 8;
	mailbox_message[4] = 8;
	mailbox_message[5] = 0x3;    // device = USB
	mailbox_message[6] = 0x0;	 // 1 = off	
	mailbox_message[7] = 0x0;

	mailbox_write(MB_CHANNEL_TAGS, ARMaddrToGPUaddr(&mailbox_message[0]));
	mailbox_read(MB_CHANNEL_TAGS);

	if ((mailbox_message[1] == 0x80000000) && (mailbox_message[4] == 0x80000008)) {
		return OK;
	}
	return ErrorDevice;												// Failed to turn on
}

/*-INTERNAL: HCDReset--------------------------------------------------------
 Does a softstart on core and uses ARM timer tick to timeout if neccessary.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDReset(void) {
	uint64_t original_tick;

	original_tick = timer_getTickCount64();							// Hold original tickcount
	do {
		if (tick_difference(original_tick, timer_getTickCount64())> 100000) {
			return ErrorTimeout;									// Return timeout error
		}
	} while (DWC_CORE_RESET->AhbMasterIdle == false);				// Keep looping until idle or timeout

	DWC_CORE_RESET->CoreSoft = true;								// Reset the soft core

	volatile struct CoreReset temp;
	original_tick = timer_getTickCount64();							// Hold original tickcount
	do {
		if (tick_difference(original_tick, timer_getTickCount64())> 100000) {
			return ErrorTimeout;									// Return timeout error
		}
		temp = *DWC_CORE_RESET;										// Read reset register
	} while (temp.CoreSoft == true || temp.AhbMasterIdle == false); // Keep looping until soft reset low/idle high or timeout

	return OK;														// Return success
}

/*-INTERNAL: HCDTransmitFifoFlush-------------------------------------------
 Flushes TX fifo buffers again uses ARM timer tick to timeout if neccessary.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDTransmitFifoFlush(enum CoreFifoFlush fifo) {
	uint64_t original_tick;

	DWC_CORE_RESET->TransmitFifoFlushNumber = fifo;					// Set fifo flush type
	DWC_CORE_RESET->TransmitFifoFlush = true;						// Execute transmit flush

	original_tick = timer_getTickCount64();							// Hold original tick count
	do {
		if (tick_difference(original_tick, timer_getTickCount64())> 100000) {
			return ErrorTimeout;									// Return timeout error
		}
	} while (DWC_CORE_RESET->TransmitFifoFlush == true);			// Loop until flush signal low or timeout

	return OK;														// Return success
}

/*-INTERNAL: HCDReceiveFifoFlush---------------------------------------------
 Flushes RX fifo buffers again uses ARM timer tick to timeout if neccessary.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDReceiveFifoFlush(void) {
	uint64_t original_tick;

	DWC_CORE_RESET->ReceiveFifoFlush = true;						// Execute recieve flush

	original_tick = timer_getTickCount64();							// Hold original tick count
	do {
		if (tick_difference(original_tick, timer_getTickCount64())> 100000) {
			return ErrorTimeout;									// Return timeout error
		}
	} while (DWC_CORE_RESET->ReceiveFifoFlush == true);				// Loop until flush signal low or timeout

	return OK;														// Return success
}

/*-INTERNAL: HCDStart--------------------------------------------------------
 Starts the HCD system once completed this routiune the system is operational.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
/* BackGround:  ULPI was developed by a group of USB industry leaders to   */
/* address the need for low - cost USB and OTG. Existing specifications    */
/* including UTMI and UTMI + were developed primarily for Macrocell(IP)    */
/* development, and are not optimized for use as an external PHY.          */
/* Using the existing UTMI + specification as a starting point, the ULPI   */
/* working group reduced the number of interface signals to 12 pins, with  */
/* an optional implementation of 8 pins.The package size of PHY and Link   */
/* IC’s are drastically reduced. This not only lowers the cost of Link and */
/* PHY IC’s, but also makes for a smaller PCB.							   */
/*-------------------------------------------------------------------------*/
RESULT HCDStart (void) {
	RESULT result;
	struct UsbControl coreUsb;

	coreUsb = *DWC_CORE_CONTROL;									// Read core control register
	coreUsb.UlpiDriveExternalVbus = 0;								// ULPI bit UseExternalVbusIndicator set to 0
	coreUsb.TsDlinePulseEnable = 0;									// Dline pulsing set to zero
	*DWC_CORE_CONTROL = coreUsb;									// Write control register

	LOG_DEBUG("HCD: Master reset.\n");								
	if ((result = HCDReset()) != OK) {								// Attempt a HCD reset which will soft reset the USB core
		LOG("FATAL ERROR: Could not do a Master reset on HCD.\n");	// Log the fatal error
		return result;												// Return fail result
	}

	if (!PhyInitialised) {											// If physical interface hasn't been initialized
		LOG_DEBUG("HCD: One time phy initialisation.\n");
		PhyInitialised = true;										// Read that we have done this one time call
		coreUsb = *DWC_CORE_CONTROL;								// Read core control register
		coreUsb.ModeSelect = UTMI;									// We will bring up UTMI+ interface .. no ULPI
		LOG_DEBUG("HCD: Interface: UTMI+.\n");						
		coreUsb.PhyInterface = false;								// Take existing phy interface down .. I assume
		*DWC_CORE_CONTROL = coreUsb;								// Write control register
		if ((result = HCDReset()) != OK) {							// You need to do a soft reset to make those settings happen
			LOG("FATAL ERROR: Could not do a Master reset on HCD.\n");// Log the fatal error
			return result;											// Return fail result
		}
	}

	coreUsb = *DWC_CORE_CONTROL;									// Read control again after possible reset above									
	if (DWC_CORE_HARDWARE->HighSpeedPhysical == Ulpi
		&& DWC_CORE_HARDWARE->FullSpeedPhysical == Dedicated) {
		LOG_DEBUG("HCD: ULPI FSLS configuration: enabled.\n");	
		coreUsb.UlpiFsls = true;								
		coreUsb.ulpi_clk_sus_m = true;
	} else {
		LOG_DEBUG("HCD: ULPI FSLS configuration: disabled.\n");
		coreUsb.UlpiFsls = false;
		coreUsb.ulpi_clk_sus_m = false;
	}
	*DWC_CORE_CONTROL = coreUsb;									// Write control register

	struct CoreAhb tempAhb;
	tempAhb = *DWC_CORE_AHB;										// Read the AHB register
	tempAhb.DmaEnable = true;										// Set the DMA on
	tempAhb.DmaRemainderMode = Incremental;							// DMA remainders that aren't aligned use incremental 
	*DWC_CORE_AHB = tempAhb;										// Write the AHB register

	coreUsb = *DWC_CORE_CONTROL;									// Read control register ... again	
	switch (DWC_CORE_HARDWARE->OperatingMode) {						// Switch based on capabilities read from hardware
	case HNP_SRP_CAPABLE:
		LOG_DEBUG("HCD: HNP/SRP configuration: HNP, SRP.\n");
		coreUsb.HnpCapable = true;
		coreUsb.SrpCapable = true;
		break;
	case SRP_ONLY_CAPABLE:
	case SRP_CAPABLE_DEVICE:
	case SRP_CAPABLE_HOST:
		LOG_DEBUG("HCD: HNP/SRP configuration: SRP.\n");
		coreUsb.HnpCapable = false;
		coreUsb.SrpCapable = true;
		break;
	case NO_HNP_SRP_CAPABLE:
	case NO_SRP_CAPABLE_DEVICE:
	case NO_SRP_CAPABLE_HOST:
		LOG_DEBUG("HCD: HNP/SRP configuration: none.\n");
		coreUsb.HnpCapable = false;
		coreUsb.SrpCapable = false;
		break;
	}
	*DWC_CORE_CONTROL = coreUsb;									// Write control register 
	LOG_DEBUG("HCD: Core started.\n");
	LOG_DEBUG("HCD: Starting host.\n");

	DWC_POWER_AND_CLOCK->Raw32 = 0;									// Release any power or clock halts given the bit names 

	if (DWC_CORE_HARDWARE->HighSpeedPhysical == Ulpi
		&& DWC_CORE_HARDWARE->FullSpeedPhysical == Dedicated
		&& coreUsb.UlpiFsls) {										// ULPI FsLs Host mode must have 48Mhz clock
		LOG_DEBUG("HCD: Host clock: 48Mhz.\n");
		DWC_HOST_CONFIG->ClockRate = Clock48MHz;					// Select 48Mhz clock
	} else {
		LOG_DEBUG("HCD: Host clock: 30-60Mhz.\n");
		DWC_HOST_CONFIG->ClockRate = Clock30_60MHz;					// Select 30-60Mhz clock
	}

	DWC_HOST_CONFIG->FslsOnly = true;								// ULPI FsLs Host mode, I assume other mode is ULPI only  .. documentation would be nice

	*DWC_CORE_RECEIVESIZE = ReceiveFifoSize;						// Set recieve fifo size

	DWC_CORE_NONPERIODICFIFO->Size.Depth = NonPeriodicFifoSize;		// Set non-periodic fifo depth
	DWC_CORE_NONPERIODICFIFO->Size.StartAddress = ReceiveFifoSize;	// Set non-periodic start address

	DWC_CORE_PERIODICINFO->HostSize.Depth = PeriodicFifoSize;		// Set periodic fifo depth
	DWC_CORE_PERIODICINFO->HostSize.StartAddress = ReceiveFifoSize + NonPeriodicFifoSize; // Set periodic start address

	LOG_DEBUG("HCD: Set HNP: enabled.\n");

	struct CoreOtgControl tempOtgControl;
	tempOtgControl = *DWC_CORE_OTGCONTROL;							// Read the OTG register
	tempOtgControl.HostSetHnpEnable = true;							// Enable the host
	*DWC_CORE_OTGCONTROL = tempOtgControl;							// Write the Otg register

	if ((result = HCDTransmitFifoFlush(FlushAll)) != OK)			// Flush the transmit FIFO
		return result;												// Return error source if fatal fail
	if ((result = HCDReceiveFifoFlush()) != OK)						// Flush the recieve FIFO
		return result;												// Return error source if fatal fail

	if (!DWC_HOST_CONFIG->EnableDmaDescriptor) {
		for (int channel = 0; channel < DWC_CORE_HARDWARE->HostChannelCount; channel++) {
			struct HostChannelCharacteristic tempChar;
			tempChar = DWC_HOST_CHANNEL[channel].Characteristic;	// Read and hold characteristic	
			tempChar.channel_enable = false;						// Clear host channel enable
			tempChar.channel_disable = true;						// Set host channel disable
			tempChar.endpoint_direction = USB_DIRECTION_IN;			// Set direction to in/read
			DWC_HOST_CHANNEL[channel].Characteristic = tempChar;	// Write the characteristics
		}

		// Halt channels to put them into known state.
		for (int channel = 0; channel < DWC_CORE_HARDWARE->HostChannelCount; channel++) {
			struct HostChannelCharacteristic tempChar;
			tempChar = DWC_HOST_CHANNEL[channel].Characteristic;	// Read and hold characteristic	
			tempChar.channel_enable = true;							// Set host channel enable
			tempChar.channel_disable = true;						// Set host channel disable
			tempChar.endpoint_direction = USB_DIRECTION_IN;			// Set direction to in/read
			DWC_HOST_CHANNEL[channel].Characteristic = tempChar;	// Write the characteristics

			uint64_t original_tick;
			original_tick = timer_getTickCount64();					// Hold original timertick
			do {
				if (tick_difference(original_tick, timer_getTickCount64()) > 0x100000) {
					LOG("HCD: Unable to clear halt on channel %i.\n", channel);
				}
			} while (DWC_HOST_CHANNEL[channel].Characteristic.channel_enable);// Repeat until goes enabled or timeout
		}
	}

	struct HostPort tempPort;
	tempPort = *DWC_HOST_PORT;										// Fetch host port 
	if (!tempPort.Power) {
		LOG_DEBUG("HCD: Initial power physical host up.\n");
		tempPort.Raw32 &= HOSTPORTMASK;								// Cleave off all the temp bits	
		tempPort.Power = true;										// Set the power bit
		*DWC_HOST_PORT = tempPort;									// Write value to port
	}

	LOG_DEBUG("HCD: Initial resetting physical host.\n");
	tempPort = *DWC_HOST_PORT;										// Fetch host port 
	tempPort.Raw32 &= HOSTPORTMASK;									// Cleave off all the temp bits	
	tempPort.Reset = true;											// Set the reset bit
	*DWC_HOST_PORT = tempPort;										// Write value to port
	timer_wait(60000);												// 60ms delay
	tempPort = *DWC_HOST_PORT;										// Fetch host port 
	tempPort.Raw32 &= HOSTPORTMASK;									// Cleave off all the temp bits	
	tempPort.Reset = false;											// Clear the reset bit
	*DWC_HOST_PORT = tempPort;										// Write value to port

	LOG_DEBUG("HCD: Successfully started.\n");

	return OK;														// Return success
}


/*==========================================================================}
{				   INTERNAL HOST TRANSMISSION ROUTINES					    }
{==========================================================================*/

/*-INTERNAL: HCDCheckErrorAndAction -----------------------------------------
 Given a channel interrupt flags and whether packet was complete (not split)
 it will set sendControl structure with what to do next.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDCheckErrorAndAction(struct ChannelInterrupts interrupts, bool packetSplit, struct UsbSendControl* sendCtrl) {
	sendCtrl->ActionResendSplit = false;							// Make sure resend split flag is cleared
	sendCtrl->ActionRetry = false;									// Make sure retry flag is cleared
	/* First deal with all the fatal errors .. no use dealing with trivial errors if these are set */
	if (interrupts.AhbError) {										// Ahb error signalled .. which means packet size too large
		sendCtrl->ActionFatalError = true;							// This is a fatal error the packet size is all wrong
		return ErrorDevice;											// Return error device
	}
	if (interrupts.DataToggleError) {								// In bulk tranmission endpoint is supposed to toggle between data0/data1
		sendCtrl->ActionFatalError = true;							// Pretty sure this is a fatal error you can't fix it by resending
		return ErrorTransmission;									// Transmission error
	}
	/* Next deal with the fully successful case  ... we can return OK */
	if (interrupts.Acknowledgement) {								// Endpoint device acknowledge
		if (interrupts.TransferComplete) sendCtrl->Success = true;	// You can set the success flag
			else sendCtrl->ActionResendSplit = true;				// Action is to try sending split again
		sendCtrl->GlobalTries = 0;
		return OK;													// Return OK result
	}
	/* Everything else is minor error invoking a retry .. so first update counts */
	if (packetSplit) {
		sendCtrl->SplitTries++;										// Increment split tries as we have a split packet
		if (sendCtrl->SplitTries == 5) {							// Ridiculous number of split resends reached .. fatal error
			sendCtrl->ActionFatalError = true;						// This is a fatal error something is very wrong
			return ErrorTransmission;								// Transmission error
		}
		sendCtrl->ActionResendSplit = true;							// Action is to try sending split again
	} else {
		sendCtrl->PacketTries++;									// Increment packet tries as packet was not split
		if (sendCtrl->PacketTries == 3) {							// Ridiculous number of packet resends reached .. fatal error
			sendCtrl->ActionFatalError = true;						// This is a fatal error something is very wrong
			return ErrorTransmission;								// Transmission error
		}
		sendCtrl->ActionRetry = true;								// Action is to try sending the packet again
	}
	/* Check no transmission errors and if so deal with minor cases */
	if (!interrupts.Stall && !interrupts.BabbleError &&
		!interrupts.FrameOverrun) {									// No transmission error
		/* If endpoint NAK nothing wrong just demanding a retry */
		if (interrupts.NegativeAcknowledgement)						// Endpoint device NAK ..nothing wrong
			return ErrorTransmission;								// Simple tranmission error .. resend
		/* Next deal with device not ready case */
		if (interrupts.NotYet)
			return ErrorTransmission;								// Endpoint device not yet ... resend
		return ErrorTimeout;										// Let guess program just timed out
	}
	/* Everything else updates global count as it is serious */
	sendCtrl->GlobalTries++;										// Increment global tries
																	/* If global tries reaches 3 .. its a fatal error */
	if (sendCtrl->GlobalTries == 3) {								// Global tries has reached 3
		sendCtrl->ActionRetry = false;								// Clear retry action flag .. it's fatal
		sendCtrl->ActionResendSplit = false;						// Clear retyr sending split again .. it's fatal
		sendCtrl->ActionFatalError = true;							// This is a fatal error to many global errors
		return ErrorTransmission;									// Transmission error
	}
	/* Deal with stall */	
	if (interrupts.Stall) {											// Stall signalled .. device endpoint problem
		return ErrorStall;											// Return the stall error
	}
	/* Deal with true transmission errors */
	if ((interrupts.BabbleError) ||									// Bable error is a packet transmission problem
		(interrupts.FrameOverrun) ||								// Frame overrun error means stop bit failed at packet end
		(interrupts.TransactionError))								
	{
		return ErrorTransmission;									// Transmission error
	}
	return ErrorGeneral;											// If we get here then no idea why error occured (probably program error)
}

/*-INTERNAL: HCDWaitOnTransmissionResult------------------------------------
 When not using Interrupts, Timers or OS this is the good old polling wait
 around for transmission packet sucess or timeout. HCD supports multiple
 options on sending the packets this static polled is just one way.
 19Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDWaitOnTransmissionResult(uint32_t timeout, uint8_t channel, struct ChannelInterrupts *IntFlags) {
	struct ChannelInterrupts tempInt;
	uint64_t original_tick = timer_getTickCount64();					// Hold original tick count
	do {
		timer_wait(100);
		if (tick_difference(original_tick, timer_getTickCount64()) > timeout) {
			if (IntFlags) *IntFlags = tempInt;						// Return interrupt flags if requested					
			return ErrorTimeout;									// Return timeout error
		}
		tempInt = DWC_HOST_CHANNEL[channel].Interrupt;				// Read and hold interterrupt
		if (tempInt.Halt) break;									// If halted exit loop
	} while (1);													// Loop until timeout or halt signal
	if (IntFlags) *IntFlags = tempInt;								// Return interrupt flags if requested	
	return OK;														// Return success
}

/*-INTERNAL: HCDChannelTransfer----------------------------------------------
 Sends/recieves data from the given buffer and size directed by pipe settings.
 19Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDChannelTransfer(const struct UsbPipe pipe, const struct UsbPipeControl pipectrl, uint8_t* buffer, uint32_t bufferLength, enum PacketId packetId) 
{
	RESULT result;
	struct ChannelInterrupts tempInt;
	struct UsbSendControl sendCtrl = { 0 };							// Zero send control structure
	uint32_t offset = 0;											// Zero transfer position 
	uint16_t maxPacketSize;
	if (pipectrl.Channel > DWC_CORE_HARDWARE->HostChannelCount) {
		LOG("HCD: Channel %d is not available on this host.\n", pipectrl.Channel);
		return ErrorArgument;
	}
	// Convert to number
	maxPacketSize = SizeToNumber(pipe.MaxSize);						// Convert pipe packet size to integer

	/* Clear all existing interrupts. */
	DWC_HOST_CHANNEL[pipectrl.Channel].Interrupt.Raw32 = 0xFFFFFFFF;// Clear all interrupts
	DWC_HOST_CHANNEL[pipectrl.Channel].InterruptMask.Raw32 = 0x0;   // Clear all interrupt masks

	/* Program the channel. */
	struct HostChannelCharacteristic tempChar = { 0 };
	tempChar.device_address = pipe.Number;							// Set host channel address
	tempChar.endpoint_number = pipe.EndPoint;						// Set host channel endpoint
	tempChar.endpoint_direction = pipectrl.Direction;				// Set host channel direction
	tempChar.low_speed = pipe.Speed == USB_SPEED_LOW ? true : false;// Set host channel speed
	tempChar.endpoint_type = pipectrl.Type;							// Set host channel packet type
	tempChar.max_packet_size = maxPacketSize;						// Set host channel max packet size
	tempChar.channel_enable = false;								// Clear enable host channel
	tempChar.channel_disable = false;								// Clear disable host channel
	DWC_HOST_CHANNEL[pipectrl.Channel].Characteristic = tempChar;	// Write those value to host characteristics

	/* Clear and setup split control to low speed devices */
	struct HostChannelSplitControl tempSplit = { 0 };
	if (pipe.Speed != USB_SPEED_HIGH) {								// If not high speed
		LOG_DEBUG("Setting split control, addr: %i port: %i, packetSize: PacketSize: %i\n",
			pipe.lowSpeedNodePoint, pipe.lowSpeedNodePort, maxPacketSize);
		tempSplit.split_enable = true;								// Enable split
		tempSplit.hub_address = pipe.lowSpeedNodePoint;				// Set the hub address to act as node
		tempSplit.port_address = pipe.lowSpeedNodePort;				// Set the hub port address
	}
	DWC_HOST_CHANNEL[pipectrl.Channel].SplitCtrl = tempSplit;		// Write channel split control

	/* Set transfer size. */
	struct HostTransferSize tempXfer = { 0 };
	tempXfer.size = bufferLength;									// Set transfer length
	if (pipe.Speed == USB_SPEED_LOW) tempXfer.packet_count = (bufferLength + 7) / 8;
	else tempXfer.packet_count = (bufferLength + maxPacketSize - 1) / maxPacketSize;
	if (tempXfer.packet_count == 0) tempXfer.packet_count = 1;		// Make sure packet count is not zero
	tempXfer.packet_id = packetId;									// Set the packet ID
	DWC_HOST_CHANNEL[pipectrl.Channel].TransferSize = tempXfer;		// Set the transfer size

	sendCtrl.PacketTries = 0;										// Zero packet tries
	do {

		// Clear any left over channel interrupts
		DWC_HOST_CHANNEL[pipectrl.Channel].Interrupt.Raw32 = 0xFFFFFFFF;
		DWC_HOST_CHANNEL[pipectrl.Channel].InterruptMask.Raw32 = 0x0;

		// Clear any left over split
		tempSplit = DWC_HOST_CHANNEL[pipectrl.Channel].SplitCtrl;	// Read split control register
		tempSplit.complete_split = false;							// Clear complete split
		DWC_HOST_CHANNEL[pipectrl.Channel].SplitCtrl = tempSplit;	// Write split register back

		// Check if the buffer is 4 byte aligned
		if (((uint32_t)(intptr_t)&buffer[offset] & 3) != 0) {

			// Since our buffer is unaligned for OUT endpoints, copy the data
			// From the buffer to the aligned buffer
			if (pipectrl.Direction == USB_DIRECTION_OUT)
			{
				memcpy(&aligned_bufs[pipectrl.Channel], &buffer[offset], bufferLength-offset);
			}

			// The buffer isnt align 4 so use the aligned buffer for this channel transfer
			// C gets a little bit quirky because I have deferenced using the array of the structure .. help C out
			*(uint32_t*)&DWC_HOST_CHANNEL[pipectrl.Channel].DmaAddr = ARMaddrToGPUaddr(&aligned_bufs[pipectrl.Channel]);
		}
		else {
			// The buffer is 4 byte aligned so we can just use it 
			// C gets a little bit quirky because I have deferenced using the array of the structure .. help C out 
			*(uint32_t*)&DWC_HOST_CHANNEL[pipectrl.Channel].DmaAddr = ARMaddrToGPUaddr(&buffer[offset]);
		}
		
		/* Launch transmission */
		tempChar = DWC_HOST_CHANNEL[pipectrl.Channel].Characteristic;// Read host channel characteristic
		tempChar.packets_per_frame = 1;								// Set 1 frame per packet
		tempChar.channel_enable = true;								// Set enable channel
		tempChar.channel_disable = false;							// Clear channel disable
		DWC_HOST_CHANNEL[pipectrl.Channel].Characteristic = tempChar;// Write channel characteristic

		// Polling wait on transmission only option right now .. other options soon :-)
		if (HCDWaitOnTransmissionResult(5000, pipectrl.Channel, &tempInt) != OK) {
			LOG("HCD: Request on channel %i has timed out.\n", pipectrl.Channel);// Log the error
			return ErrorTimeout;									// Return timeout error
		}

		tempSplit = DWC_HOST_CHANNEL[pipectrl.Channel].SplitCtrl;	// Fetch the split details
		result = HCDCheckErrorAndAction(tempInt,
			tempSplit.split_enable, &sendCtrl);						// Check transmisson RESULT and set action flags
		if (result) LOG("Result: %i Action: 0x%08x tempInt: 0x%08x tempSplit: 0x%08x Bytes sent: %i\n",
			result, (unsigned int)sendCtrl.Raw32, (unsigned int)tempInt.Raw32, 
			(unsigned int)tempSplit.Raw32, result ? 0 : DWC_HOST_CHANNEL[pipectrl.Channel].TransferSize.size);
		if (sendCtrl.ActionFatalError) return result;				// Fatal error occured we need to bail

		sendCtrl.SplitTries = 0;									// Zero split tries count
		while (sendCtrl.ActionResendSplit) {						// Decision was made to resend split
			/* Clear channel interrupts */
			DWC_HOST_CHANNEL[pipectrl.Channel].Interrupt.Raw32 = 0xFFFFFFFF;
			DWC_HOST_CHANNEL[pipectrl.Channel].InterruptMask.Raw32 = 0x0;

			/* Set we are completing the split */
			tempSplit = DWC_HOST_CHANNEL[pipectrl.Channel].SplitCtrl;
			tempSplit.complete_split = true;						// Set complete split flag
			DWC_HOST_CHANNEL[pipectrl.Channel].SplitCtrl = tempSplit;

			/* Launch transmission */
			tempChar = DWC_HOST_CHANNEL[pipectrl.Channel].Characteristic;
			tempChar.channel_enable = true;
			tempChar.channel_disable = false;
			DWC_HOST_CHANNEL[pipectrl.Channel].Characteristic = tempChar;

			// Polling wait on transmission only option right now .. other options soon :-)
			if (HCDWaitOnTransmissionResult(5000, pipectrl.Channel, &tempInt) != OK) {
				LOG("HCD: Request split completion on channel:%i has timed out.\n", pipectrl.Channel);// Log error
				return ErrorTimeout;								// Return timeout error
			}

			tempSplit = DWC_HOST_CHANNEL[pipectrl.Channel].SplitCtrl;// Fetch the split details again
			result = HCDCheckErrorAndAction(tempInt,
				tempSplit.split_enable, &sendCtrl);					// Check RESULT of split resend and set action flags
			//if (result) LOG("Result: %i Action: 0x%08lx tempInt: 0x%08lx tempSplit: 0x%08lx Bytes sent: %i\n",
			//	result, sendCtrl.RawUsbSendContol, tempInt.RawInterrupt, tempSplit.RawSplitControl, RESULT ? 0 : DWC_HOST_CHANNEL[pipectrl.Channel].TransferSize.TransferSize);
			if (sendCtrl.ActionFatalError) return result;			// Fatal error occured bail
			if (sendCtrl.LongerDelay) timer_wait(10000);			// Not yet response slower delay
				else timer_wait(2500);								// Small delay between split resends
		}

		if (sendCtrl.Success) {										// Send successful adjust buffer position
			unsigned int this_transfer;
			this_transfer = DWC_HOST_CHANNEL[pipectrl.Channel].TransferSize.size;
			
			if (((uint32_t)(intptr_t)&buffer[offset] & 3) != 0) {	// Buffer address is unaligned

				// Since our buffer is unaligned for IN endpoints
				// Copy the data from the the aligned buffer to the buffer
				// We know the aligned buffer was used because it is unaligned
				if (pipectrl.Direction == USB_DIRECTION_IN)
				{
					memcpy(&buffer[offset], aligned_bufs[pipectrl.Channel], this_transfer);
				}
			}

			offset = bufferLength - this_transfer;
		}

	} while (DWC_HOST_CHANNEL[pipectrl.Channel].TransferSize.packet_count > 0);// Full data not sent

	return OK;														// Return success as data must have been sent
}

/*-HCDSumbitControlMessage --------------------------------------------------
 Sends a control message to a device. Handles all necessary channel creation
 and other processing. The sequence of a control transfer is defined in the
 USB 2.0 manual section 5.5.  Success is indicated by return of OK (0) all
 other codes indicate an error.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDSumbitControlMessage (const struct UsbPipe pipe,			// Pipe structure (really just uint32_t)
								const struct UsbPipeControl pipectrl,// Pipe control structure 					
								uint8_t* buffer,					// Data buffer both send and recieve				 
								uint32_t bufferLength,				// Buffer length for send or recieve
								struct UsbDeviceRequest *request,	// USB request message
								uint32_t timeout,					// Timeout in microseconds on message
								uint32_t* bytesTransferred)			// Value at pointer will be updated with bytes transfered to/from buffer (NULL to ignore)				
{
	RESULT result;
	if (pipe.Number == RootHubDeviceNumber) {
		return HcdProcessRootHubMessage(buffer, bufferLength, request, bytesTransferred);
	}
	uint32_t lastTransfer = 0;

	// LOG("Setup phase ");
	// Setup phase
	struct UsbPipeControl intPipeCtrl = pipectrl;					// Copy the pipe control (We want channel really)										
	intPipeCtrl.Type = USB_TRANSFER_TYPE_CONTROL;					// Set pipe to control	
	intPipeCtrl.Direction = USB_DIRECTION_OUT;						// Set pipe to out
	if ((result = HCDChannelTransfer(pipe, intPipeCtrl,
		(uint8_t*)request, 8, USB_PID_SETUP)) != OK) {				// Send the 8 byte setup request packet
		LOG("HCD: SETUP packet to device: %#x req: %#x req Type: %#x Speed: %i PacketSize: %i LowNode: %i LowPort: %i Error: %i\n",
			pipe.Number, request->Request, request->Type, pipe.Speed, pipe.MaxSize, pipe.lowSpeedNodePoint, pipe.lowSpeedNodePort, result);// Some parameter issue
		return OK;
	}
	// LOG("Transfer phase ");
	// Data transfer phase
	if (buffer != NULL) {											// Buffer must be valid for any transfer to occur
		intPipeCtrl.Direction = pipectrl.Direction;					// Set pipe direction as requested	
		if ((result = HCDChannelTransfer(pipe, intPipeCtrl,
			&buffer[0],	bufferLength, USB_PID_DATA1)) != OK) {		// Send or recieve the data
			LOG("HCD: Could not transfer DATA to device %i.\n",
				pipe.Number);										// Log error
			return OK;
		}
		if (pipectrl.Direction == USB_DIRECTION_IN) {				// In bound pipe as per original
			lastTransfer = bufferLength - DWC_HOST_CHANNEL[pipectrl.Channel].TransferSize.size;
		}
		else {
			lastTransfer = bufferLength;							// Success so transfer is full buffer for send 
		}
	}

	//LOG("Status phase ");
	// Status phase		
	intPipeCtrl.Direction = ((bufferLength == 0) || pipectrl.Direction == USB_DIRECTION_OUT) ? USB_DIRECTION_IN : USB_DIRECTION_OUT;
	if ((result = HCDChannelTransfer(pipe, intPipeCtrl, &buffer[0], 0, USB_PID_DATA1)) != OK)	// Send or recieve the status
	{
		LOG("HCD: Could not transfer STATUS to device %i.\n",
			pipe.Number);											// Log error
		return OK;
	}
	if (DWC_HOST_CHANNEL[pipectrl.Channel].TransferSize.size != 0)
		LOG_DEBUG("HCD: Warning non zero status transfer! %d.\n", DWC_HOST_CHANNEL[pipectrl.Channel].TransferSize.size);

	if (bytesTransferred) *bytesTransferred = lastTransfer;
	//LOG("\n");
	return OK;
}

/*-HCDSetAddress ------------------------------------------------------------
 Sets the address of the device with control endpoint given by the pipe. Zero
 is a restricted address for the rootHub and will return if attempted.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDSetAddress (const struct UsbPipe pipe,					// Pipe structure (really just uint32_t)
					  uint8_t channel,								// Channel to use
					  uint8_t address)								// Address to set
{
	RESULT result;
	struct UsbPipeControl pipectrl = {
		.Channel = channel,											// Use given channel channel
		.Type = USB_TRANSFER_TYPE_CONTROL,							// Control packet
		.Direction = USB_DIRECTION_OUT,								// We are writing to host
	};
	if (address == 0) return ErrorArgument;							// You can't set address zero that is strictly reserved for roothub
	result = HCDSumbitControlMessage(
		pipe,														// Pipe which points to current device endpoint
		pipectrl,													// Pipe control
		NULL,														// No data its a command
		0,															// Zero size transfer as no data
		&(struct UsbDeviceRequest) {
			.Request = SetAddress,									// Set address request
			.Type = 0,
			.Value = address,										// Address to set
		},
		ControlMessageTimeout, NULL);
	return result;													// Return the result
}

/*-INTERNAL: HCDSetConfiguration---------------------------------------------
 Sets a given USB device configuration to the config index number requested.
 28Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDSetConfiguration (struct UsbPipe pipe, uint8_t channel, uint8_t configuration) {
	RESULT result;
	struct UsbPipeControl pipectrl = {
		.Channel = channel,											// Use given channel
		.Type = USB_TRANSFER_TYPE_CONTROL,							// Control packet
		.Direction = USB_DIRECTION_OUT,								// We are writing to host
	};
	result = HCDSumbitControlMessage(
		pipe,
		pipectrl,
		NULL,
		0,
		&(struct UsbDeviceRequest) {
			.Request = SetConfiguration,							// Set configuration
			.Type = 0,
			.Value = configuration,									// Config index
		},
		ControlMessageTimeout,
		NULL);														// Read the requested configuration
	return result;													// Return result
}

/*==========================================================================}
{		 INTERNAL HCD MESSAGE ROUTINES SPECIFICALLY FOR HUB DEVICES		    }
{==========================================================================*/

/*-INTERNAL: HCDReadHubPortStatus--------------------------------------------
 Reads the given port status on a hub device. Port input is index 1 and so
 requesting port 0 is interpretted as you want the port gateway node status.
 When reading a port the return is really a HubPortFullStatus, while for
 port = 0 the return will be a struct HubFullStatus. There are uint32_t unions
 on those two structures to pass the raw 32 bits in/out.
 21Mar17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDReadHubPortStatus (const struct UsbPipe pipe,				// Control pipe to the hub 
							 uint8_t port,							// Port to get status  OR  0 = Gateway node
							 uint32_t *Status)						// HubPortFullStatus or HubFullStatus .. use Raw union  
{
	RESULT result;
	uint32_t transfer = 0;
	struct UsbPipeControl pipectrl = {
		.Channel = dwc_get_free_channel(),							// Find first free channel
		.Type = USB_TRANSFER_TYPE_CONTROL,							// Control packet
		.Direction = USB_DIRECTION_IN,								// We are reading to host
	};
	if (Status == NULL) return ErrorArgument;						// Make sure return pointer is valid
	if ((result = HCDSumbitControlMessage(
		pipe,														// Pass control pipe thru unchanged
		pipectrl,
		(uint8_t*)Status,											// Pass in pointer to status
		sizeof(uint32_t),											// We want full structure for either call which is 32 bits
		&(struct UsbDeviceRequest) {								// Construct a USB request
			.Request = GetStatus,									// Get status id
			.Type = port ? bmREQ_PORT_STATUS : bmREQ_HUB_STATUS,	// Request bit mask is for hub if port = 0, hub port otherwise 
			.Index = port,											// Port number is index 1 so we add one
			.Length = sizeof(uint32_t),								// We want full structure size
		},
		ControlMessageTimeout,										// Standard control message timeouts
		&transfer)) != OK)											// We will check transfer size so pass in pointer to our local
	{
		dwc_release_channel(pipectrl.Channel);						// Release the channel
		LOG("HCD Hub read status failed on device: %i, port: %i, Result: %#x, Pipe Speed: %#x, Pipe MaxPacket: %#x\n",
			pipe.Number, port, result, pipe.Speed, pipe.MaxSize);	// Log any error
		return result;												// Return error result
	}
	dwc_release_channel(pipectrl.Channel);							// Release the channel
	if (transfer < sizeof(uint32_t)) {								// Hub did not read amount requested
		LOG("HUB: Failed to read hub device:%i port:%i status\n",
			pipe.Number, port);										// Log error
		return ErrorDevice;											// Some quirk in enumeration usually
	}
	return OK;														// Return success
}

/*-INTERNAL: HCDChangeHubPortFeature-----------------------------------------
 Changes a feature setting on the given port on a hub device. Port input is
 index 1 and so requesting port 0 is interpretted as you are changing the
 feature on the port gateway node.
 21Mar17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDChangeHubPortFeature (const struct UsbPipe pipe,			// Control pipe to the hub 
								enum HubPortFeature feature,		// Which feature to change
								uint8_t port,						// Port to change feature  OR  0 = Gateway node
								bool set)							// Set or clear the feature
{
	RESULT result;
	struct UsbPipeControl pipectrl = {
		.Channel = dwc_get_free_channel(),							// Find first free channel
		.Type = USB_TRANSFER_TYPE_CONTROL,							// Control packet
		.Direction = USB_DIRECTION_OUT,								// We are writing to host
	};
	if ((result = HCDSumbitControlMessage(
		pipe,														// Pipe settings passed thru as is
		pipectrl,
		NULL,														// No buffer as no data
		0,															// Length zero as no data
		&(struct UsbDeviceRequest) {
			.Request = set ? SetFeature : ClearFeature,				// Set or clear feature as requested
			.Type = port ? bmREQ_PORT_FEATURE : bmREQ_HUB_FEATURE,	// Request bit mask is for hub if port = 0, hub port otherwise
			.Value = (uint16_t)feature,								// Feature we are changing
			.Index = port,											// Port (index 1 so add one)
		},
		ControlMessageTimeout,										// Standard control message timeouts
		NULL)) != OK)												// Ignore transfer pointer as zero data
	{
		dwc_release_channel(pipectrl.Channel);						// Release the channel
		LOG("HUB: Failed to change port feature for device: %i, Port:%d feature:%d set:%d.\n",
			pipe.Number, port, feature, set);						// Log any error
		return result;												// Return error result
	}
	dwc_release_channel(pipectrl.Channel);							// Release the channel
	return OK;														// Return success
}


/*==========================================================================}
{      INTERNAL FUNCTIONS THAT OPERATE TO GET DESCRIPTORS FROM DEVICES	    }
{==========================================================================*/

/*-INTERNAL: HCDReadStringDescriptor-----------------------------------------
 Reads the string descriptor at the given string index returning an ascii of
 the descriptor. Internally the descriptor is unicode so the raw descriptor
 is not returned. The code is setup to US English language support (0x409),
 and if a string does not have a valid English language string the default
 language is use to read blindly to satisfy enumeration. Non english speakers
 if you want to choose a different language you need to change 0x409 in the
 code below to your standard USB language ID you want.
 21Mar17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDReadStringDescriptor (struct UsbPipe pipe,				// Control pipe to the USB device
								uint8_t stringIndex,				// String index to be returned
								char* buffer,						// Pointer to a buffer
								uint32_t length)					// The size of that buffer
{
	RESULT result;
	uint32_t transfer = 0;
	struct UsbDescriptorHeader Header __attribute__((aligned(4)));	// aligned for DMA transfer a discriptor header is two bytes
	char descBuffer[256] __attribute__((aligned(4)));				// aligned for DMA transfer a descriptor is max 256 bytes (uint8_t size in header definition)
	uint16_t langIds[96] __attribute__((aligned(4))) = { 0 };		// aligned for DMA transfer a descriptors
	bool NoEnglishSupport = false;									// Preset no english support false

	if (buffer == NULL || stringIndex == 0) return ErrorArgument;	// Make sure values valid
	result = HCDGetDescriptor(pipe, USB_DESCRIPTOR_TYPE_STRING, 0, 0, &langIds, 2,
		bmREQ_GET_DEVICE_DESCRIPTOR, &transfer, true);				// Get language support header
	if ((result != OK) && (transfer < 2)) {							// Could not read language support data
		LOG("HCD: Could not read language support for device: %i\n",
			pipe.Number);											// Log the error
		return ErrorArgument;										// I am lost what is going on bail
	}

	// langIds 0 actually has 0x03 (string descriptor) and size of language support words .. if it doesn't bail
	if ((langIds[0] >> 8) != 0x03) {								// The top byte has to be 0x03
		LOG("HCD: Not a valid language support descriptor on device: %i\n",
			pipe.Number);											// Log the error
		return ErrorArgument;										// I am lost what is going on bail
	}
	// So we have size to read for all the language support pairs
	result = HCDGetDescriptor(pipe, USB_DESCRIPTOR_TYPE_STRING, 0, 0, &langIds, langIds[0] & 0xFF,
		bmREQ_GET_DEVICE_DESCRIPTOR, &transfer, true);				// Get all language support pair data
	if ((result != OK) && (transfer < (langIds[0] & 0xFF))) {		// We failed to read all the support data
		LOG("HCD: Could not read all the language support data on device: %i\n",
			pipe.Number);											// Log the error		
		return ErrorArgument;										// I am lost what is going on bail
	}

	// Okay lets see if 0x409 is supported .. Sorry I am only interested in english
	// Non speaking people feel free to choose you own language id for your language 
	int i;
	int lastEntry = (langIds[0] & 0xFF) >> 1;						// So from header size we can work last pair entry 
	for (i = 1; i < lastEntry; i++) {								// Remember langIds[0] is header so start at 1
		if (langIds[i] == 0x409) break;								// English id pair exists yipee
	}
	if (i == lastEntry) {											// No search all pairs no english support available
		LOG("No english language string available on device: %i\n",
			pipe.Number);											// Log the error
		NoEnglishSupport = true;									// Set that flag
	}

	// Pull header of string descriptor so we get size. If no english available use lang pair at position 1
	// We have to read string descriptor for enumeration .. but we don't have to put it in buffer
	result = HCDGetDescriptor(pipe, USB_DESCRIPTOR_TYPE_STRING, stringIndex,
		NoEnglishSupport ? langIds[1] : 0x409, &Header,
		sizeof(struct UsbDescriptorHeader), bmREQ_GET_DEVICE_DESCRIPTOR, 
		&transfer, true);											// Read string descriptor header only
	if ((result != OK) || (transfer != sizeof(struct UsbDescriptorHeader))) {
		LOG("HCD: Could not fetch string descriptor header (%i) for device: %i\n",
			stringIndex, pipe.Number);								// Log the error
		return ErrorDevice;											// No idea what problem is so bail										
	}

	// Okay we got the size of the string so now read the entire size
	result = HCDGetDescriptor(pipe, USB_DESCRIPTOR_TYPE_STRING, stringIndex,
		NoEnglishSupport ? langIds[1] : 0x409, &descBuffer,
		Header.DescriptorLength, bmREQ_GET_DEVICE_DESCRIPTOR, 
		&transfer, true);											// Read the full string 	
	if ((result != OK) || (transfer != Header.DescriptorLength)) {
		LOG("HCD: Could not fetch string descriptor (%i) for device: %i\n",
			stringIndex, pipe.Number);								// Log the error
		return ErrorArgument;										// No idea what problem is so bail
	}

	// Finally we need to turn the UTF16 string back to ascii for caller
	i = 0;															// Set i to zero in case no english support
	if (NoEnglishSupport == false) {								// Yipee we have english support				
		uint16_t* p = (uint16_t*)&descBuffer[2];					// Start of unicode text .. 2 bytes at top are descriptor header
		for (i = 0; i < ((Header.DescriptorLength - 2) >> 1)
			&& (i < length - 1); i++) buffer[i] = wctob(*p++);		// Narrow character from unicode to ascii											
	}
	buffer[i] = '\0';												// Make asciiz

	return OK;														// Return success
}

/*==========================================================================}
{					   INTERNAL HOST CONTROL FUNCTIONS					    }
{==========================================================================*/

/*-INTERNAL: HCDInitialise---------------------------------------------------
 Initialises the hardware that is in use. This usually means powering up that
 hardware and it may therefore need a set delay between this call and  the
 HCDStart routine after which you can use the system.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDInitialise(void) {
	uint32_t VendorId = *DWC_CORE_VENDORID;							// Read the vendor ID
	uint32_t UserId = *DWC_CORE_USERID;								// Read the user ID
	if ((VendorId & 0xfffff000) != 0x4f542000) {					// 'OT'2 
		LOG("HCD: Hardware: %c%c%x.%x%x%x (BCM%.5x). Driver incompatible. Expected OT2.xxx (BCM2708x).\n",
			(char)((VendorId >> 24) & 0xff), (char)((VendorId >> 16) & 0xff),
			(unsigned int)((VendorId >> 12) & 0xf), (unsigned int)((VendorId >> 8) & 0xf),
			(unsigned int)((VendorId >> 4) & 0xf), (unsigned int)((VendorId >> 0) & 0xf),
			(unsigned int)((UserId >> 12) & 0xFFFFF));
		return ErrorIncompatible;
	} else {
		LOG("HCD: Hardware: %c%c%x.%x%x%x (BCM%.5x).\n",
			(char)((VendorId >> 24) & 0xff),(char)((VendorId >> 16) & 0xff),
			(unsigned int)((VendorId >> 12) & 0xf), (unsigned int)((VendorId >> 8) & 0xf),
			(unsigned int)((VendorId >> 4) & 0xf), (unsigned int)((VendorId >> 0) & 0xf),
			(unsigned int)((UserId >> 12) & 0xFFFFF));
	}

	if (DWC_CORE_HARDWARE->Architecture != InternalDma) {			// We only allow DMA transfer
		LOG("HCD: Host architecture does not support Internal DMA\n");
		return ErrorIncompatible;									// Return hardware incompatible
	}

	if (DWC_CORE_HARDWARE->HighSpeedPhysical == NotSupported) {		// We need high speed transfers
		LOG("HCD: High speed physical unsupported\n");
		return ErrorIncompatible;									// Return hardware incompatible
	}

	struct CoreAhb tempAhb = *DWC_CORE_AHB;							// Read the AHB register to temp
	tempAhb.InterruptEnable = false;								// Clear interrupt enable bit
	*DWC_CORE_AHB = tempAhb;										// Write temp back to AHB register
	DWC_CORE_INTERRUPTMASK->Raw32 = 0;								// Clear all interrupt masks

	if (PowerOnUsb() != OK) {										// Power up the USB hardware
		LOG("HCD: Failed to power on USB Host Controller.\n");		// Log failed to start power up
		return ErrorIncompatible;									// Return hardware incompatible
	}
	return OK;														// Return success
}

/*==========================================================================}
{      INTERNAL FUNCTIONS THAT ADD AND REMOCE HID PAYLOADS TO DEVICES	    }
{==========================================================================*/

/*-INTERNAL: AddHidPayload---------------------------------------------------
 Makes sure the device has no other sorts of payload AKA it's simple node
 and if so will find the first free hid storage area and attach it as a hid
 payload.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT AddHidPayload (struct UsbDevice *device) {
	if (device && device->PayLoadId == NoPayload) {					// Check device is valid and not already assigned a payload
		for (int number = 0; number < MaximumHids; number++) {		// Search each entry in hid data payload array
			if (HidTable[number].MaxHID == 0) {						// Find first free entry
				device->HidPayload = &HidTable[number];				// Place pointer to the device payload pointer
				device->PayLoadId = HidPayload;						// Set the payload id
				HidTable[number].MaxHID = MaxHIDPerDevice;			// Preset maximum HID's per device (signals in use)
				return OK;											// Return success
			}
		}
		return ErrorMemory;											// Too many hids ... no free hid table entries 
	}
	return ErrorArgument;											// Passed an invalid device ... programming error 
}

/*-INTERNAL: RemoveHidPayload------------------------------------------------
 Makes sure the hid payload is free from device will make it free again in the
 hid table to be allocated again.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
void RemoveHidPayload(struct UsbDevice *device) {
	if (device && device->PayLoadId == HidPayload && device->HidPayload) {// Check device is valid, is assigned a hid payload and the hidpayload is valid
		memset(device->HidPayload, 0, sizeof(struct HidDevice));	// Clear all the hid payload data which will mark it unused
		device->HidPayload = NULL;									// Payload removed from device
		device->PayLoadId = NoPayload;								// Clear payload ID its gone
	}
}

/*==========================================================================}
{      INTERNAL FUNCTIONS THAT ADD AND REMOCE HUB PAYLOADS TO DEVICES	    }
{==========================================================================*/

/*-INTERNAL: AddHubPayload---------------------------------------------------
 Makes sure the device has no other sorts of payload AKA it's simple node
 and if so will find the first free hub storage area and attach it as a hub
 payload.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT AddHubPayload(struct UsbDevice *device) {
	if (device && device->PayLoadId == NoPayload) {					// Check device is valid and not already assigned a payload
		for (int number = 0; number < MaximumHubs; number++) {		// Search each entry in hub data payload array
			if (HubTable[number].MaxChildren == 0) {				// Find first free entry
				device->HubPayload = &HubTable[number];				// Place pointer to the device payload pointer
				device->PayLoadId = HubPayload;						// Set the payload id
				HubTable[number].MaxChildren = MaxChildrenPerDevice;// Max children starts out as set by us (hub may shorten up itself) .. non zero means entry in use
				return OK;											// Return success
			}
		}
		return ErrorMemory;											// Too many hubs ... no free hub table entries 
	}
	return ErrorArgument;											// Passed an invalid device ... programming error 
}

/*-INTERNAL: RemoveHubPayload------------------------------------------------
 Makes sure the hub payload is free of all children and then clears payload
 which will make it free again in the hub table to be allocated again.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
void UsbDeallocateDevice(struct UsbDevice *device);					// UsbDeallocate and RemoveHubPayload call each other so we need a forward declare
void RemoveHubPayload(struct UsbDevice *device) {
	if (device && device->PayLoadId == HubPayload && device->HubPayload) {// Check device is valid, is assigned a hub payload and the hubpayload is valid
		for (int i = 0; i < device->HubPayload->MaxChildren; i++) {	// Check each of the children (we would hope already done but check)
			if (device->HubPayload->Children[i])					// If a child is valid
				UsbDeallocateDevice(device->HubPayload->Children[i]);// Any valid children need to be deallocated
		}
		memset(device->HubPayload, 0, sizeof(struct HubDevice));	// Clear all the hub payload data which will mark it unused
		device->HubPayload = NULL;									// Payload removed from device
		device->PayLoadId = NoPayload;								// Clear payload ID its gone
	}
}

/*==========================================================================}
{       INTERNAL FUNCTIONS THAT ADD/DETACH AND DEALLOCATE DEVICES		    }
{==========================================================================*/

/*-INTERNAL: UsbAllocateDevice-----------------------------------------------
 Find first free device entry table and return that pointer as our device.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT UsbAllocateDevice(struct UsbDevice **device) {
	if (device) {
		for (int number = 0; number < MaximumDevices; number++) {	// Search device table entries
			if (DeviceTable[number].PayLoadId == 0) {				// Find first free entry (PayloadId goes to non zero when in use)
				*device = &DeviceTable[number];						// Return that entry area as device
				(*device)->Pipe0.Number = number + 1;				// Our device Id is the table entry we found
				(*device)->Config.Status = USB_STATUS_ATTACHED;		// Set status to attached
				(*device)->ParentHub.PortNumber = 0;				// Start on port 0
				(*device)->ParentHub.Number = 0xFF;					// At this stage we have no parent
				(*device)->PayLoadId = NoPayload;					// Set PayLoadId to no payload attached (PayloadId goes non zero indicating in use)
				(*device)->HubPayload = NULL;						// Make sure payload pointer is NULL
				return OK;											// Return success
			}
		}
		return ErrorMemory;											// All device table entries are in use .. no free table
	}
	return ErrorArgument;											// The device pointer was invalid .. serious programming error								
}

/*-INTERNAL: UsbDeallocateDevice---------------------------------------------
 Deallocate a device releasing all memory associated to the device
 11Feb17 LdB
 --------------------------------------------------------------------------*/
void UsbDeallocateDevice (struct UsbDevice *device) {
	if (IsHub(device->Pipe0.Number)) {								// If this device is a hub we will need to deal with the children
		/* A hub must deallocate all its children first */
		for (int i = 0; i < device->HubPayload->MaxChildren; i++) {	// For each child
			if (device->HubPayload->Children[i] != NULL)			// If that child is valid
				UsbDeallocateDevice(device->HubPayload->Children[i]);// Iterate deallocating each child
		}
		RemoveHubPayload(device);									// Having disposed of the children we need to get rid of the hub payload	
	}
	if (device->ParentHub.Number < MaximumDevices) {				// Check we have a valid parent
		struct UsbDevice* parent;
		parent = &DeviceTable[device->ParentHub.Number-1];			// Fetch the parent hub device
		/* Now remove this device from any parent .. check everything to make sure it is a child */
		if (parent->PayLoadId == HubPayload && parent->HubPayload &&// Check we have a valid parent and it is a hub
			device->ParentHub.PortNumber < parent->HubPayload->MaxChildren && // Check we are on a valid port
			parent->HubPayload->Children[device->ParentHub.PortNumber] == device)// Check we are the child pointer on that port
			parent->HubPayload->Children[device->ParentHub.PortNumber] = NULL;// Yes we really are the child so clear our entry
	}
	memset(device, 0, sizeof(struct UsbDevice));					// Clear the device entry area which will mark it unused
}

/*==========================================================================}
{			    NON HCD INTERNAL HUB FUNCTIONS ON PORTS						}
{==========================================================================*/
RESULT HubPortReset(struct UsbDevice *device, uint8_t port) {
	RESULT result;
	struct HubPortFullStatus portStatus;
	uint32_t retry, timeout;
	if (!IsHub(device->Pipe0.Number)) return ErrorDevice;			// If device is not a hub then bail
	LOG_DEBUG("HUB: Reseting device: %i Port: %d. source: %i\n", device->Pipe0.Number, port, source);
	for (retry = 0; retry < 3; retry++) {
		if ((result = HCDChangeHubPortFeature(device->Pipe0,
			FeatureReset, port + 1, true)) != OK) 					// Issue a setfeature of reset
		{
			LOG("HUB: Device %i Failed to reset Port%d.\n",
				device->Pipe0.Number, port + 1);					// Log any failure
			return result;											// Return result that is causing failure
		}
		timeout = 0;
		do {
			timer_wait(20000);
			if ((result = HCDReadHubPortStatus(device->Pipe0, port + 1, &portStatus.Raw32)) != OK) {
				LOG("HUB: Hub failed to get status (4) for %s.Port%d.\n", UsbGetDescription(device), port + 1);
				return result;
			}
			timeout++;
		} while (!portStatus.Change.ResetChanged && !portStatus.Status.Enabled && timeout < 10);

		if (timeout == 10) continue;

		LOG_DEBUG("HUB: %s.Port%d Status %x:%x.\n", UsbGetDescription(device), port + 1, portStatus.RawStatus, portStatus.RawChange);

		if (portStatus.Change.ConnectedChanged || !portStatus.Status.Connected)
			return ErrorDevice;

		if (portStatus.Status.Enabled)
			break;
	}

	if (retry == 3) {
		LOG("HUB: Cannot enable %s.Port%d. Please verify the hardware is working.\n", UsbGetDescription(device), port + 1);
		return ErrorDevice;
	}

	if ((result = HCDChangeHubPortFeature(device->Pipe0, FeatureResetChange, port + 1, false)) != OK) {
		LOG("HUB: Failed to clear reset on %s.Port%d.\n", UsbGetDescription(device), port + 1);
	}
	return OK;
}

/*-INTERNAL: HubPortConnectionChanged ---------------------------------------
 If a connection on a port on a hub as changed this routine is called to deal
 with the change. This will involve it enumerating an added new device or the
 deallocation of a removed or detached device.
 21Mar17 LdB
 --------------------------------------------------------------------------*/
RESULT EnumerateDevice (struct UsbDevice *device, struct UsbDevice* ParentHub, uint8_t PortNum); // We need to forward declare
RESULT HubPortConnectionChanged(struct UsbDevice *device, uint8_t port) {
	RESULT result;
	struct HubDevice *data;
	struct HubPortFullStatus portStatus;
	if (!IsHub(device->Pipe0.Number)) return ErrorDevice;

	data = device->HubPayload;

	if ((result = HCDReadHubPortStatus(device->Pipe0, port + 1, &portStatus.Raw32)) != OK) {
		LOG("HUB: Hub failed to get status (2) for %s.Port%d.\n", UsbGetDescription(device), port + 1);
		return result;
	}
	LOG_DEBUG("HUB: %s.Port%d Status %x:%x.\n", UsbGetDescription(device), port + 1, portStatus.RawStatus, portStatus.RawChange);

	if ((result = HCDChangeHubPortFeature(device->Pipe0, FeatureConnectionChange, port + 1, false)) != OK) {
		LOG("HUB: Failed to clear change on %s.Port%d.\n", UsbGetDescription(device), port + 1);
	}

	if ((!portStatus.Status.Connected && !portStatus.Status.Enabled) || data->Children[port] != NULL) {
		LOG("HUB: Disconnected %s.Port%d - %s.\n", UsbGetDescription(device), port + 1, UsbGetDescription(data->Children[port]));
		UsbDeallocateDevice(data->Children[port]);
		data->Children[port] = NULL;
		if (!portStatus.Status.Connected) return OK;
	}

	if ((result = HubPortReset(device, port)) != OK) {
		LOG("HUB: Could not reset %s.Port%d for new device.\n", UsbGetDescription(device), port + 1);
		return result;
	}

	if ((result = UsbAllocateDevice(&data->Children[port])) != OK) {
		LOG("HUB: Could not allocate a new device entry for %s.Port%d.\n", UsbGetDescription(device), port + 1);
		return result;
	}

	if ((result = HCDReadHubPortStatus(device->Pipe0, port + 1, &portStatus.Raw32)) != OK) {
		LOG("HUB: Hub failed to get status (3) for %s.Port%d.\n", UsbGetDescription(device), port + 1);
		return result;
	}

	LOG_DEBUG("HUB: %s. Device:%i Port:%d Status %04x:%04x.\n", UsbGetDescription(device), device->Pipe0.Number, port, portStatus.RawStatus, portStatus.RawChange);

	if (portStatus.Status.HighSpeedAttatched) data->Children[port]->Pipe0.Speed = USB_SPEED_HIGH;
	else if (portStatus.Status.LowSpeedAttatched) {
		data->Children[port]->Pipe0.Speed = USB_SPEED_LOW;
		data->Children[port]->Pipe0.lowSpeedNodePoint = device->Pipe0.Number;
		data->Children[port]->Pipe0.lowSpeedNodePort = port;
	}
	else data->Children[port]->Pipe0.Speed = USB_SPEED_FULL;
	data->Children[port]->ParentHub.Number = device->Pipe0.Number;
	data->Children[port]->ParentHub.PortNumber = port;
	if ((result = EnumerateDevice(data->Children[port], device, port)) != OK) {
		LOG("HUB: Could not connect to new device in %s.Port%d. Disabling.\n", UsbGetDescription(device), port + 1);
		UsbDeallocateDevice(data->Children[port]);
		data->Children[port] = NULL;
		if (HCDChangeHubPortFeature(device->Pipe0, FeatureEnable, port + 1, false) != OK) {
			LOG("HUB: Failed to disable %s.Port%d.\n", UsbGetDescription(device), port + 1);
		}
		return result;
	}
	return OK;
}


/*-HubCheckConnection -------------------------------------------------------
 Checks device is a hub and if a valid hub checks connection status of given
 port on the hub. If it has changed performs necessary actions such as the
 enumerating of a new device or deallocating an old one.
 10Apr17 LdB
 --------------------------------------------------------------------------*/
RESULT HubCheckConnection(struct UsbDevice *device, uint8_t port) {
	RESULT result;
	struct HubPortFullStatus portStatus;
	struct HubDevice *data;

	if (!IsHub(device->Pipe0.Number)) return ErrorDevice;
	data = device->HubPayload;

	if ((result = HCDReadHubPortStatus(device->Pipe0, port + 1, &portStatus.Raw32)) != OK) {
		if (result != ErrorDisconnected)
			LOG("HUB: Failed to get hub port status (1) for %s.Port%d.\n", UsbGetDescription(device), port + 1);
		return result;
	}

	if (portStatus.Change.ConnectedChanged) {
		LOG_DEBUG("Device %i, Port: %i changed\n", device->Pipe0.Number, port);
		HubPortConnectionChanged(device, port);
	}

	if (portStatus.Change.EnabledChanged) {
		if (HCDChangeHubPortFeature(device->Pipe0, FeatureEnableChange, port + 1, false) != OK) {
			LOG("HUB: Failed to clear enable change %s.Port%d.\n", UsbGetDescription(device), port + 1);
		}

		// This may indicate EM interference.
		if (!portStatus.Status.Enabled && portStatus.Status.Connected && data->Children[port] != NULL) {
			LOG("HUB: %s.Port%d has been disabled, but is connected. This can be cause by interference. Reenabling!\n", UsbGetDescription(device), port + 1);
			HubPortConnectionChanged(device, port);
		}
	}

	if (portStatus.Status.Suspended) {
		if (HCDChangeHubPortFeature(device->Pipe0, FeatureSuspend, port + 1, false) != OK) {
			LOG("HUB: Failed to clear suspended port - %s.Port%d.\n", UsbGetDescription(device), port + 1);
		}
	}

	if (portStatus.Change.OverCurrentChanged) {
		if (HCDChangeHubPortFeature(device->Pipe0, FeatureOverCurrentChange, port + 1, false) != OK) {
			LOG("HUB: Failed to clear over current port - %s.Port%d.\n", UsbGetDescription(device), port + 1);
		}
	}

	if (portStatus.Change.ResetChanged) {
		if (HCDChangeHubPortFeature(device->Pipe0, FeatureResetChange, port + 1, false) != OK) {
			LOG("HUB: Failed to clear reset port - %s.Port%d.\n", UsbGetDescription(device), port + 1);
		}
	}

	return OK;
}

/*-INTERNAL: HubCheckForChange ----------------------------------------------
 This performs an iteration loop to check each port on each hub to see if any
 device has been added or removed.
 21Mar17 LdB
 --------------------------------------------------------------------------*/
void HubCheckForChange(struct UsbDevice *device) {
	if (IsHub(device->Pipe0.Number)) {
		for (int i = 0; i < device->HubPayload->MaxChildren; i++) {
			if (HubCheckConnection(device, i) != OK) continue;		// If port is not connected move to next port
			if (device->HubPayload->Children[i] != NULL)			// If child device is valid
				HubCheckForChange(device->HubPayload->Children[i]);	// Iterate this call
		}
	}
}

/*==========================================================================}
{						 INTERNAL ENUMERATION ROUTINES						}
{==========================================================================*/

/*-INTERNAL: EnumerateHID------------------------------------------------------
 If normal device enumeration detects a hid device, after normal single node
 enumeration it will call this procedure to enumerate connected HID devices.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT EnumerateHID (const struct UsbPipe pipe, struct UsbDevice *device) {
	volatile uint8_t Hi;
	volatile uint8_t Lo;
	uint8_t Buf[1024];
	for (int i = 0; i < device->HidPayload->MaxHID; i++) {
		Hi = *(uint8_t*)&device->HidPayload->Descriptor[i].HidVersionHi; // ARM7/8 alignment issue
		Lo = *(uint8_t*)&device->HidPayload->Descriptor[i].HidVersionLo; // ARM7/8 alignment issue
		int interface = device->HidPayload->HIDInterface[i];
		LOG("HID details: Version: %4x, Language: %i Descriptions: %i, Type: %i, Protocol: %i, NumInterface: %i\n",
			(unsigned int)((uint32_t)Hi << 8 | Lo),
			device->HidPayload->Descriptor[i].Countrycode,
			device->HidPayload->Descriptor[i].DescriptorCount,
			device->HidPayload->Descriptor[i].Type,
			device->Interfaces[interface].Protocol,
			device->Interfaces[interface].Number);

		if (HIDReadDescriptor(pipe.Number, i, &Buf[0], sizeof(Buf)) == OK) {
			LOG_DEBUG("HID REPORT> Page usage: 0x%02x%02x, Usage: 0x%02x%02x, Collection: 0x%02x%02x\n",
				Buf[0], Buf[1], Buf[2], Buf[3], Buf[4], Buf[5]);
			LOG_DEBUG("Bytes: %02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x\n",
				Buf[6], Buf[7], Buf[8], Buf[9], Buf[10], Buf[11], Buf[12], Buf[13], Buf[14], Buf[15], Buf[16], Buf[17], Buf[18], Buf[19], Buf[20], Buf[21],
				Buf[22], Buf[23], Buf[24], Buf[25], Buf[26], Buf[27], Buf[28], Buf[29], Buf[30], Buf[31], Buf[32], Buf[33], Buf[34], Buf[35], Buf[36], Buf[37]);
			LOG_DEBUG("Bytes: %02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x:%02x\n",
				Buf[38], Buf[39], Buf[40], Buf[41], Buf[42], Buf[43], Buf[44], Buf[45], Buf[46], Buf[47], Buf[48], Buf[49], Buf[50], Buf[51]);
		}
	}
	return OK;														// Return success
}

/*-INTERNAL: EnumerateHub ---------------------------------------------------
 Continues enumeration of each port if an enumerated detected device is a hub
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT EnumerateHub (struct UsbDevice *device) {
	RESULT result;
	uint32_t transfer;
	struct HubDevice *data;
	struct HubFullStatus status;

	if ((result = AddHubPayload(device)) != OK) {					// We are a hub so we need a hub payload
		LOG("Could not allocate hub payload, Error ID %i\n", result);
		return result;												// We must have to fouled up device allocation code
	}

	data = device->HubPayload;										// Hub payload data added grab pointer to it we will be using it a fair bit

	for (int i = 0; i < MaxChildrenPerDevice; i++)
		data->Children[i] = NULL;									// For safety make sure all children pointers are NULL

	result = HCDGetDescriptor(device->Pipe0, USB_DESCRIPTOR_TYPE_HUB, 
		0, 0, &data->Descriptor, sizeof(struct HubDescriptor),
		bmREQ_GET_HUB_DESCRIPTOR, &transfer, true);					// Fetch the HUB descriptor and hold in the hub payload, we use it a bit so saves USB bus
	if ((result != OK) || (transfer != sizeof(struct HubDescriptor)))
	{
		LOG("HCD: Could not fetch hub descriptor for device: %i\n",
			device->Pipe0.Number);									// Log the error
		return ErrorDevice;											// No idea what problem is so bail
	}
	LOG_DEBUG("Hub device %i has %i ports\n", device->Pipe0.Number, data->Descriptor.PortCount);
	LOG_DEBUG("HUB: Hub power to good: %dms.\n", data->Descriptor.PowerGoodDelay * 2);
	LOG_DEBUG("HUB: Hub current required: %dmA.\n", data->Descriptor.MaximumHubPower * 2);

	if (data->Descriptor.PortCount > MaxChildrenPerDevice) {		// Check number of ports on hub vs maxium number we allow on a hub payload
		LOG("HUB device:%i is too big for this driver to handle. Only the first %d ports will be used.\n",
			device->Pipe0.Number, MaxChildrenPerDevice);			// Log error			
	}
	else data->MaxChildren = data->Descriptor.PortCount;			// Reduce number of children down to same as hub supports

	if ((result = HCDReadHubPortStatus(device->Pipe0, 0, &status.Raw32)) != OK) // Gateway node status
	{
		LOG("HUB device:%i failed to get hub status.\n",
			device->Pipe0.Number);									// Log error
		return result;												// Return error result
	}

	LOG_DEBUG("HUB: Hub powering ports on.\n");
	for (int i = 0; i < data->MaxChildren; i++) {					// For each port
		if (HCDChangeHubPortFeature(device->Pipe0, FeaturePower,
			i + 1, true) != OK)										// Power the port							
			LOG("HUB: device: %i could not power Port%d.\n",
				device->Pipe0.Number, i + 1);						// Log error
	}
	timer_wait(data->Descriptor.PowerGoodDelay * 2000);				// Every hub has a different power stability delay

	for (int port = 0; port < data->MaxChildren; port++) {			// Now check for new device to enumerate on each port
		HubCheckConnection(device, port);							// Run connection check on each port
	}

	return OK;														// Return success
}


/*-INTERNAL: EnumerateDevice ------------------------------------------------
 All detected devices start enumeration here. We recover critical information
 of every USB device and hold those details in the device data block. Finally 
 if the device is recognized as any of the sepcial specific class then it will
 call extended enumeration for those specific classes.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT EnumerateDevice(struct UsbDevice *device, struct UsbDevice* ParentHub, uint8_t PortNum) {
	RESULT result;
	uint8_t address;
	uint32_t transferred;
	struct usb_device_descriptor desc __attribute__((aligned(4))) = { 0 };	// Device descriptor DMA aligned
	char buffer[256] __attribute__((aligned(4)));					// Text buffer

	/* Store the unique address until it is actually assigned. */
	address = device->Pipe0.Number;									// Hold unique address we will set device to
	device->Pipe0.Number = 0;										// Initially it starts as zero
	/*	 USB ENUMERATION BY THE BOOK, STEP 1 = Read first 8 Bytes of Device Descriptor	*/
	device->Pipe0.MaxSize = Bits8;									// Set max packet size to 8 ( So exchange will be exactly 1 packet)
	struct UsbPipeControl pipectrl = {
		.Channel = dwc_get_free_channel(),							// Find first free channel
		.Type = USB_TRANSFER_TYPE_CONTROL,							// Control packet
		.Direction = USB_DIRECTION_IN,								// We are reading to host
	};

	result = HCDSumbitControlMessage(
		device->Pipe0,												// Pipe as given to us
		pipectrl,													// Pipe control structure
		(uint8_t*)&desc,											// Pointer to descriptor
		8,															// Ask for first 8 bytes as per USB specification
		&(struct UsbDeviceRequest) {								// We will build a request structure
			.Request = GetDescriptor,								// We want a descriptor obviously
			.Type = bmREQ_GET_DEVICE_DESCRIPTOR,					// Recipient is a flag usually 0x0 for normal device, 0x20 for a hub
			.Value = (uint16_t)USB_DESCRIPTOR_TYPE_DEVICE << 8,		// Type and the index (0) get compacted as the value
			.Index = 0,												// We want descriptor 0
			.Length = 8,											// 8 bytes as per USB enumeration by the book
		},
		ControlMessageTimeout,										// The standard timeout for any control message
		&transferred);												// Pass in pointer to get bytes transferred back
	if ((result != OK) || (transferred != 8)) {						// This should pass on any valid device
		dwc_release_channel(pipectrl.Channel);						// Release the channel we are exiting 
		LOG("Enumeration: Step 1 on device %i failed, Result: %#x.\n",
			address, result);										// Log any error
		return result;												// Fatal enumeration error of this device
	}
	device->Pipe0.MaxSize = SizeFromNumber(desc.bMaxPacketSize0);	// Set the maximum endpoint packet size to pipe from response
	device->Config.Status = USB_STATUS_DEFAULT;						// Move device enumeration to default

	/*	USB ENUMERATION BY THE BOOK STEP 2 = Reset Port (old device support)	*/
	if (ParentHub != NULL) {										// Roothub is the only one who will have a NULL parent and you can't reset a FAKE hub
		// Reset the port for what will be the second time.
		if ((result = HubPortReset(ParentHub, PortNum)) != OK) {
			dwc_release_channel(pipectrl.Channel);					// Release the channel we are exiting
			LOG("HCD: Failed to reset port again for new device %s.\n", UsbGetDescription(device));
			device->Pipe0.Number = address;
			return result;
		}
	}
	
	/*			USB ENUMERATION BY THE BOOK STEP 3 = Set Device Address			*/
	if ((result = HCDSetAddress(device->Pipe0, pipectrl.Channel, address)) != OK) {
		dwc_release_channel(pipectrl.Channel);					   // Release the channel we are exiting
		LOG("Enumeration: Failed to assign address to %#x.\n", address);// Log the error
		device->Pipe0.Number = address;								// Set device number just so it stays valid
		return result;												// Fatal enumeration error of this device
	}
	device->Pipe0.Number = address;									// Device successfully addressed so put it back to control pipe								
	timer_wait(10000);												// Allows time for address to propagate.
	device->Config.Status = USB_STATUS_ADDRESSED;					// Our enumeration status in now addressed

	/*	USB ENUMERATION BY THE BOOK STEP 4 = Read Device Descriptor At Address	*/
	result = HCDGetDescriptor(
		device->Pipe0,												// Device control 0 pipe
		USB_DESCRIPTOR_TYPE_DEVICE,							        // Fetch device descriptor 
		0,															// Index 0
		0,															// Language 0
		&device->Descriptor,										// Pointer to buffer in device structure 
		sizeof(device->Descriptor),									// Ask for entire descriptor
		bmREQ_GET_DEVICE_DESCRIPTOR,								// Recipient device
		&transferred, true);										// Pass in pointer to get bytes transferred back
	if ((result != OK) || (transferred != sizeof(device->Descriptor))) {// This should pass on any valid device
		dwc_release_channel(pipectrl.Channel);						// Release the channel we are exiting
		LOG("Enumeration: Step 4 on device %i failed, Result: %#x.\n",
			device->Pipe0.Number, result);							// Log any error
		return result;												// Fatal enumeration error of this device
	}
	LOG_DEBUG("Device: %i, Class: %i\n", device->Pipe0.Number, device->Descriptor.Class);


	/*		USB ENUMERATION BY THE BOOK STEP 5 = Read Device Configurations		*/
	// Read the master Config at index 0 ... this is not really a config but an index to avail configs
	uint32_t transfer;
	struct usb_configuration_descriptor configDesc __attribute__((aligned(4)));// aligned for DMA transfer 
	result = HCDGetDescriptor(device->Pipe0, USB_DESCRIPTOR_TYPE_CONFIGURATION, 0, 0,
		&configDesc, sizeof(configDesc), bmREQ_GET_DEVICE_DESCRIPTOR,
		&transfer, true);											// Read the config descriptor 	
	if ((result != OK) || (transfer != sizeof(configDesc))) {
		dwc_release_channel(pipectrl.Channel);						// Release the channel we are exiting
		LOG("HCD: Error: %i, reading configuration descriptor for device: %i\n",
			result, device->Pipe0.Number);							// Log the error
		return ErrorDevice;											// No idea what problem is so bail
	}
	device->Config.ConfigStringIndex = configDesc.iConfiguration;	// Grab string index while here

	// Most devices I played with only have 1 config .. regardless we will take first
	// The index to call is given as at offset 5 bConfigurationValue
	// Read it by that index it's probably the same but just do it
	uint8_t configNum = configDesc.bConfigurationValue;
	// Okay we have the total length of config so we will read it in entirity
	uint8_t configBuffer[1024];										// Largest config I have ever seen is few hundred bytes this is 1K buffer
	result = HCDSumbitControlMessage(
		device->Pipe0,												// Device 
		pipectrl,											        // Create pipe control structure 
		&configBuffer[0],											// Buffer pointer passed in as is
		configDesc.wTotalLength,									// Length of whole config descriptor
		&(struct UsbDeviceRequest) {								// We will build a request structure
			.Request = GetDescriptor,								// We want a descriptor obviously
			.Type = bmREQ_GET_DEVICE_DESCRIPTOR,					// We want normal device descriptor
			.Value = (uint16_t)USB_DESCRIPTOR_TYPE_CONFIGURATION << 8,// Type and the index get compacted as the value
			.Index = 0,												// Language ID is the index
			.Length = configDesc.wTotalLength,						// Duplicate the length
		},
		ControlMessageTimeout,										// The standard timeout for any control message
		&transfer);													// Set pointer to fetch transfer bytes
	if ((result != OK) || (transfer != configDesc.wTotalLength)) {	// Check if anything went wrong
		dwc_release_channel(pipectrl.Channel);						// Release the channel
		LOG("HCD: Failed to read configuration descriptor for device %i, %u bytes read, Error: %i.\n",
			device->Pipe0.Number, (unsigned int)transfer, result);				// Log error
		if (result != OK) return result;							// Return error result
		return ErrorDevice;											// Something went badly wrong .. bail
	}

	// So now we need to search for interfaces and endpoints
	uint8_t EndPtCnt = 0;											// Preset endpoint count to zero
	uint8_t hidCount = 0;											// Preset hid count to zero
	uint32_t i = 0;													// Start array search at zero
	while (i < configDesc.wTotalLength - 1) {						// So while we havent reached end of config data
		switch (configBuffer[i + 1]) {								// i will be on a descriptor header i+1 is decsriptor type 
		case USB_DESCRIPTOR_TYPE_INTERFACE: {						// Ok we have an interface descriptor we need to add it
			myMemCopy((uint8_t*)&device->Interfaces[device->MaxInterface],
				&configBuffer[i], 
				sizeof(struct UsbInterfaceDescriptor));				// configBuffer[i] is descriptor size as well as first byte
			device->MaxInterface++;									// One interface added
			EndPtCnt = 0;											// Reset endpoint count to zero (we are on new interface now)
			break;
		}
		case USB_DESCRIPTOR_TYPE_ENDPOINT: {						// Ok we have an endpoint descriptor we need to add it
			myMemCopy((uint8_t*)&device->Endpoints[device->MaxInterface - 1][EndPtCnt], 
				&configBuffer[i],
				sizeof(struct UsbEndpointDescriptor));				// configBuffer[i] is descriptor size as well as first byte
			EndPtCnt++;												// One endpoint added so move index
			break;
		}
		case USB_DESCRIPTOR_TYPE_HID: {								// HID Interface found
			if (hidCount == 0) {									// First HID descriptor found
				if ((result = AddHidPayload(device)) != OK) {		// Ok so we need to add a hid payload to device
					dwc_release_channel(pipectrl.Channel);			// Release the channel we are exiting
					LOG("Could not allocate hid payload, Error ID %i\n", result);
					return result;									// We must have to fouled up device allocation code
				};
			}
			if (hidCount < MaxHIDPerDevice) {						// We can hold a limited sane number of HID descriptors
				myMemCopy((uint8_t*)&device->HidPayload->Descriptor[hidCount],
					&configBuffer[i], sizeof(struct HidDescriptor));// Copy descriptor to HID data block
				device->HidPayload->HIDInterface[hidCount] = device->MaxInterface - 1; // Hold the interface the HID is on
				hidCount++;											// Add one to HID count
			}
			if (sizeof(struct HidDescriptor) != configBuffer[i]) {
				LOG("HID Entry wrong size\n");
			}
			break;
		}
		default:
			break;
		}
		i = i + configBuffer[i];									// Add config descriptor size .. which moves us to next descriptor
	}

	/*	  USB ENUMERATION BY THE BOOK STEP 6 = Set Configuration to Device		*/
	if ((result = HCDSetConfiguration(device->Pipe0, pipectrl.Channel, configNum)) != OK) {
		dwc_release_channel(pipectrl.Channel);					   // Release the channel we are exiting
		LOG("HCD: Failed to set configuration %#x for device %i.\n",
			configNum, device->Pipe0.Number);
		return result;
	}
	device->Config.ConfigIndex = configNum;							// Hold the configuration index
	device->Config.Status = USB_STATUS_CONFIGURED;					// Set device status to configured

	LOG("HCD: Attach Device %s. Address:%d Class:%d USB:%x.%x, %d configuration(s), %d interface(s).\n",
		UsbGetDescription(device), address, device->Descriptor.bDeviceClass, (device->Descriptor.bcdUSB >> 8) & 0xFF,
		device->Descriptor.bcdUSB & 0xFF, device->Descriptor.bNumConfigurations, device->MaxInterface);
	
	if (device->Descriptor.iProduct != 0) {
		result = HCDReadStringDescriptor(device->Pipe0, device->Descriptor.iProduct, &buffer[0], sizeof(buffer));
		if (result == OK) LOG("HCD:  -Product:       %s.\n", buffer);
	}
	
	if (device->Descriptor.iManufacturer != 0) {
		result = HCDReadStringDescriptor(device->Pipe0, device->Descriptor.iManufacturer, &buffer[0], sizeof(buffer));
		if (result == OK) LOG("HCD:  -Manufacturer:  %s.\n", buffer);
	}
	if (device->Descriptor.iSerialNumber != 0) {
		result = HCDReadStringDescriptor(device->Pipe0, device->Descriptor.iSerialNumber, &buffer[0], sizeof(buffer));
		if (result == OK) LOG("HCD:  -SerialNumber:  %s.\n", buffer);
	}


	if (device->Config.ConfigStringIndex != 0) {
		result = HCDReadStringDescriptor(device->Pipe0, device->Config.ConfigStringIndex, &buffer[0], sizeof(buffer));
		if (result == OK) LOG("HCD:  -Configuration: %s.\n", buffer);
	}


	/*	     USB ENUMERATION BY THE BOOK STEP 7 = ENUMERATE SPECIAL DEVICES		*/
	if (device->Descriptor.bDeviceClass == DeviceClassHub) {		// If device is a hub then enumerate it
		if ((result = EnumerateHub(device)) != OK) {				// Run hub enumeration
			dwc_release_channel(pipectrl.Channel);					// Release the channel we are exiting
			LOG("Could not enumerate HUB device %i, Error ID %i\n",
				device->Pipe0.Number, result);						// Log error
			return result;											// Return the error
		}
	} else if (hidCount > 0) {										// HID interface on the device
		device->HidPayload->MaxHID = hidCount;						// Set the maxium HID record number
		if ((result = EnumerateHID(device->Pipe0, device)) != OK) {	// Ok so enumerate the HID device
			dwc_release_channel(pipectrl.Channel);					// Release the channel we are exiting
			LOG("Could not enumerate HID device %i, Error ID %i\n",
				device->Pipe0.Number, result);
			return result;											// return the error
		}
	}
	dwc_release_channel(pipectrl.Channel);							// Release the channel we are exiting
	return OK;
}

/*-INTERNAL: EnumerateDevice ------------------------------------------------
 This is called from USBInitialize and will allocate our fake rootHub device 
 and then begin enumeration of the whole USB bus.
 11Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT UsbAttachRootHub(void) {
	RESULT result;
	struct UsbDevice *rootHub = NULL;
	LOG_DEBUG("Allocating RootHub\n");
	if (DeviceTable[0].PayLoadId != 0)								// If RootHub is already in use
		UsbDeallocateDevice(&DeviceTable[0]);						// We will need to deallocate it and every child
	result = UsbAllocateDevice(&rootHub);							// Try allocating the root hub now
	if (rootHub != &DeviceTable[0]) result = ErrorCompiler;			// Somethign really wrong .. 1st allocation should always be DeviceList[0]
	if (result != OK) return result;								// Return error result somethging fatal happened
	DeviceTable[0].Pipe0.Speed = USB_SPEED_FULL;					// Set our fake hub to full speed .. as it's fake we cant really ask it speed can we :-)
	DeviceTable[0].Pipe0.MaxSize = Bits64;							// Set our fake hub to 64 byte packets .. as it's fake we need to do it manually
	DeviceTable[0].Config.Status = USB_STATUS_POWERED;				// Set our fake hub status to configured .. as it's fake we need to do manually
	RootHubDeviceNumber = 0;										// Roothub number is zero
	return EnumerateDevice(&DeviceTable[0], NULL, 0);				// Ok start enumerating the USB bus as roothub port 1 is the physical bus
}

/***************************************************************************}
{					      PUBLIC INTERFACE ROUTINES			                }
****************************************************************************/

/*--------------------------------------------------------------------------}
{						 PUBLIC USB DESCRIPTOR ROUTINES						}
{--------------------------------------------------------------------------*/

/*-HCDGetDescriptor ---------------------------------------------------------
 Has the ability to fetches all the different descriptors from the device if
 you provide the right parameters. It is a marshal call that many internal
 descriptor reads will use and it has no checking on parameters. So if you
 provide invalid parameters it will most likely fail and return with error.
 The descriptor is read in two calls first the header is read to check the
 type matches and it provides the descriptor size. If the buffer length is
 longer than the descriptor the second call shortens the length to just the
 descriptor length. So the call provides the length of data requested or
 shorter if the descriptor is shorter than the buffer space provided.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT HCDGetDescriptor (const struct UsbPipe pipe,					// Pipe structure to send message thru (really just uint32_t) 
						 enum usb_descriptor_type type,				// The type of descriptor
						 uint8_t index,								// The index of the type descriptor
						 uint16_t langId,							// The language id
						 void* buffer,								// Buffer to recieve descriptor
						 uint32_t length,							// Maximumlength of descriptor
						 uint8_t recipient,							// Recipient flags									 
						 uint32_t *bytesTransferred,     			// Value at pointer will be updated with bytes transfered to/from buffer (NULL to ignore)								
						 bool runHeaderCheck)						// Whether to run header check
{
	RESULT result;
	uint32_t transfer;
	struct __attribute__((aligned(4))) UsbDescriptorHeader header  = { 0 };
	struct UsbPipeControl pipectrl = {
		.Channel = dwc_get_free_channel(),							// Find first free channel
		.Type = USB_TRANSFER_TYPE_CONTROL,							// This is a control request
		.Direction = USB_DIRECTION_IN,								// In to host as we are getting
	};
	if (runHeaderCheck) {
		result = HCDSumbitControlMessage(
			pipe,													// Pipe passed in as is
			pipectrl,											    // Pipe control structure 
			(uint8_t*)&header,										// Buffer to description header
			sizeof(header),											// Size of the header
			&(struct UsbDeviceRequest) {							// We will build a request structure
				.Request = GetDescriptor,							// We want a descriptor obviously
				.Type = recipient,									// Recipient is a flag usually bmREQ_GET_DEVICE_DESCRIPTOR, bmREQ_GET_HUB_DESCRIPTOR etc
				.Value = (uint16_t)type << 8 | index,				// Type and the index get compacted as the value
				.Index = langId,									// Language ID is the index
				.Length = sizeof(header),							// Duplicate the length
			},
			ControlMessageTimeout,									// The standard timeout for any control message
			NULL);													// Ignore bytes transferred
		if ((result == OK) && (header.DescriptorType != type))
			result = ErrorGeneral;									// For some strange reason descriptor type is not right
		if (result != OK) {											// RESULT in error
			dwc_release_channel(pipectrl.Channel);					// Release the channel
			LOG("HCD: Fail to get descriptor %#x:%#x recepient: %#x, device:%i. RESULT %#x.\n",
				type, index, recipient, pipe.Number, result);		// Log any error
			return result;											// Error reading descriptor header
		}
		if (length > header.DescriptorLength)						// Check descriptor length vs buffer space
			length = header.DescriptorLength;						// The descriptor is shorter than buffer space provided
	}
	result = HCDSumbitControlMessage(
		pipe,														// Pipe passed in as is
		pipectrl,												    // Pipe control structure 
		buffer,														// Buffer pointer passed in as is
		length,														// Length transferred (it may be shorter from above)
		&(struct UsbDeviceRequest) {								// We will build a request structure
			.Request = GetDescriptor,								// We want a descriptor obviously
			.Type = recipient,										//  Recipient is a flag usually bmREQ_GET_DEVICE_DESCRIPTOR, bmREQ_GET_HUB_DESCRIPTOR etc
			.Value = (uint16_t)type << 8 | index,					// Type and the index get compacted as the value
			.Index = langId,										// Language ID is the index
			.Length = length,										// Duplicate the length
		},
		ControlMessageTimeout,										// The standard timeout for any control message
		&transfer);													// Set pointer to fetch transfer bytes
	if (length != transfer) result = ErrorTransmission; 			// The requested length does not match read length
	if (result != OK) {
		dwc_release_channel(pipectrl.Channel);						// Release the channel
		LOG("HCD: Failed to get descriptor %#x:%#x for device:%i. RESULT %#x.\n",
			type, index, pipe.Number, result);						// Log any error
	}
	dwc_release_channel(pipectrl.Channel);							// Release the channel
	if (bytesTransferred) *bytesTransferred = transfer;				// Return the bytes transferred
	return result;													// Return the result
}

/*--------------------------------------------------------------------------}
{					 PUBLIC GENERIC USB INTERFACE ROUTINES					}
{--------------------------------------------------------------------------*/

/*-UsbInitialise-------------------------------------------------------------
 Initialises the USB driver by performing necessary interfactions with the
 host controller driver, and enumerating the initial device tree.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
RESULT UsbInitialise (void) {
	RESULT result;
	chfree = (1 << DWC_NUM_CHANNELS) - 1;							// Set the channel free bit masks

	if ((result = HCDInitialise()) != OK) {							// Initialize host control driver
		LOG("FATAL ERROR: HCD failed to initialise.\n");			// Some hardware issue
		return result;												// Return any fatal error
	}

	if ((result = HCDStart()) != OK) {								// Start the host control driver						
		LOG("USBD: Abort, HCD failed to start.\n");
		return result;												// Return any fatal error
	}
	if ((result = UsbAttachRootHub()) != OK) {						// Attach the root hub .. which will launch enumeration
		LOG("USBD: Failed to enumerate devices.\n");
		return result;												// Retrn any fatal error
	}
	return OK;														// Return success
}

/*-IsHub---------------------------------------------------------------------
 Will return if the given usbdevice is infact a hub and thus has hub payload
 data available. Remember the gateway node of a hub is a normal usb device.
 You should always call this first up in any routine that accesses the hub
 payload to make sure the payload pointers are valid. If it returns true it
 is safe to proceed and do things with the hub payload via it's pointer.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
bool IsHub (uint8_t devNumber) {
	if ((devNumber > 0) && (devNumber <= MaximumDevices)) {			// Check the address is valid not zero and max devices or less
		struct UsbDevice* device = &DeviceTable[devNumber - 1];		// Shortcut to device pointer we are talking about	
		if (device->PayLoadId == HubPayload && device->HubPayload)	// It has a HUB payload ID and the HUB payload pointer is valid
			return true;											// Confirmed as a hub
	}
	return false;													// Not a hub
}

/*-IsHid---------------------------------------------------------------------
 Will return if the given usbdevice is infact a hid and thus has hid payload
 data available. Remember a hid device is a normal usb device which takes
 human input (like keyboard, mouse etc). You should always call this first
 in any routine that accesses the hid payload to make sure the pointers are
 valid. If it returns true it is safe to proceed and do things with the hid
 payload via it's pointer.
 24Feb17 LdB
--------------------------------------------------------------------------*/
bool IsHid (uint8_t devNumber) {
	if ((devNumber > 0) && (devNumber <= MaximumDevices)) {			// Check the address is valid not zero and max devices or less
		struct UsbDevice* device = &DeviceTable[devNumber - 1];		// Shortcut to device pointer we are talking about					
		if (device->PayLoadId == HidPayload && device->HidPayload)	// It has a HID payload ID and the HID payload pointer is valid
			return true;											// Confirmed as a hid
	}
	return false;													// Not a hid
}

/*-IsMassStorage------------------------------------------------------------
 Will return if the given usbdevice is infact a mass storage device and thus 
 has a mass storage payload data available. You should always call this first
 in any routine that accesses the storage payload to make sure the pointers 
 are valid. If it returns true it is safe to proceed and do things with the 
 storage payload via it's pointer.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
bool IsMassStorage (uint8_t devNumber) {
	if ((devNumber > 0) && (devNumber <= MaximumDevices)) {			// Check the address is valid not zero and max devices or less
		struct UsbDevice* device = &DeviceTable[devNumber - 1];		// Shortcut to device pointer we are talking about
		if (device->PayLoadId == MassStoragePayload &&				// Device pointer is valid and we have a payload id of mass storage
			device->MassPayload != NULL) return true;				// Confirmed as a mass storage device
	}
	return false;													// Not a mass storage device
}

/*-IsMouse-------------------------------------------------------------------
 Will return if the given usbdevice is infact a mouse. This initially checks
 the device IsHid and then refines that down to looking at the interface and
 checking it is defined as a mouse.
 24Feb17 LdB
--------------------------------------------------------------------------*/
bool IsMouse (uint8_t devNumber) {
	if ((devNumber > 0) && (devNumber <= MaximumDevices)) {			// Check the address is valid not zero and max devices or less
		struct UsbDevice* device = &DeviceTable[devNumber - 1];		// Shortcut to device pointer we are talking about
		if (device->PayLoadId == HidPayload && device->HidPayload   // Its a valid HID
		 && device->Interfaces[0].Protocol == 2) return true;		// Protocol 2 means a mouse
	}
	return false;													// Not a mouse device
}

/*-IsKeyboard----------------------------------------------------------------
 Will return if the given usbdevice is infact a keyboard. This initially will
 check the device IsHid and then refines that down to looking at the interface
 and checking it is defined as a keyboard.
 24Feb17 LdB
 --------------------------------------------------------------------------*/
bool IsKeyboard (uint8_t devNumber) {
	if ((devNumber > 0) && (devNumber <= MaximumDevices)) {			// Check the address is valid not zero and max devices or less
		struct UsbDevice* device = &DeviceTable[devNumber - 1];		// Shortcut to device pointer we are talking about
		if (device->PayLoadId == HidPayload && device->HidPayload   // Its a valid HID
			&& device->Interfaces[0].Protocol == 1) return true;	// Protocol 1 means a keyboard
	}
	return false;													// Not a mouse device
}

/*-UsbGetRootHub ------------------------------------------------------------
 On a Universal Serial Bus, there exists a root hub. This if often a virtual
 device, and typically represents a one port hub, which is the physical
 universal serial bus for this computer. It is always address 1. It is present
 to allow uniform software manipulation of the universal serial bus itself.
 This will return that FAKE rootHub or NULL on failure. Reason for failure is
 generally not having called USBInitialize to start the USB system.          
 11Apr17 LdB
 --------------------------------------------------------------------------*/
struct UsbDevice * UsbGetRootHub (void) { 
	if (DeviceTable[0].PayLoadId != 0)								// Check the root hub is in use AKA Usbinitialize was called
		return &DeviceTable[0];										// Return the rootHub AKA DeviceList[0]
	return NULL;													// Return NULL as no valid rootHub
}

/*-UsbDeviceAtAddress -------------------------------------------------------
  Given the unique USB address this will return the pointer to the USB device
  structure. If the address is not actually in use it will return NULL.
 11Apr17 LdB
 --------------------------------------------------------------------------*/
struct UsbDevice * UsbDeviceAtAddress (uint8_t devNumber) {
	if  ((devNumber > 0) && (DeviceTable[devNumber-1].PayLoadId != 0)) // Check the device address is not zero and then check that id is actually in use
		return &DeviceTable[devNumber-1];							// Return the device at the address given
	return NULL;													// Return NULL as that device address is not in use
}

/*--------------------------------------------------------------------------}
{					 PUBLIC USB CHANGE CHECKING ROUTINES					}
{--------------------------------------------------------------------------*/

/*-UsbCheckForChange --------------------------------------------------------
 Recursively calls HubCheckConnection on all ports on all hubs connected to
 the root hub. It will hence automatically change the device tree matching
 any physical changes.
 10Apr17 LdB
 --------------------------------------------------------------------------*/
void UsbCheckForChange(void) {
	if (DeviceTable[0].PayLoadId != 0)								// Check the root hub is in use AKA Usbinitialize was called
		HubCheckForChange(&DeviceTable[0]);							// Launch iterration of checking for changes from the roothub
}


/*--------------------------------------------------------------------------}
{					 PUBLIC DISPLAY USB INTERFACE ROUTINES					}
{--------------------------------------------------------------------------*/

/*-UsbGetDescription --------------------------------------------------------
 Returns a description for a device. This is not read from the device, this
 is just generated given by the driver.
 Unchanged from Alex Chadwick
 --------------------------------------------------------------------------*/
const char* UsbGetDescription (struct UsbDevice *device) {
	if (device->Config.Status == USB_STATUS_ATTACHED)
		return "New Device (Not Ready)\0";
	else if (device->Config.Status == USB_STATUS_POWERED)
		return "Unknown Device (Not Ready)\0";
	else if (device == &DeviceTable[0])
		return "USB Root Hub\0";

	switch (device->Descriptor.bDeviceClass) {
	case DeviceClassHub:
		if (device->Descriptor.bcdUSB == 0x210)
			return "USB 2.1 Hub\0";
		else if (device->Descriptor.bcdUSB == 0x200)
			return "USB 2.0 Hub\0";
		else if (device->Descriptor.bcdUSB == 0x110)
			return "USB 1.1 Hub\0";
		else if (device->Descriptor.bcdUSB == 0x100)
			return "USB 1.0 Hub\0";
		else
			return "USB Hub\0";
	case DeviceClassVendorSpecific:
		if (device->Descriptor.idVendor == 0x424 &&
			device->Descriptor.idProduct == 0xec00)
			return "SMSC LAN9512\0";
	case DeviceClassInInterface:
		if (device->Config.Status == USB_STATUS_CONFIGURED) {
			switch (device->Interfaces[0].Class) {
			case InterfaceClassAudio:
				return "USB Audio Device\0";
			case InterfaceClassCommunications:
				return "USB CDC Device\0";
			case InterfaceClassHid:
				switch (device->Interfaces[0].Protocol) {
				case 1:
					return "USB Keyboard\0";
				case 2:
					return "USB Mouse\0";
				default:
					return "USB HID\0";
				}
			case InterfaceClassPhysical:
				return "USB Physical Device\0";
			case InterfaceClassImage:
				return "USB Imaging Device\0";
			case InterfaceClassPrinter:
				return "USB Printer\0";
			case InterfaceClassMassStorage:
				return "USB Mass Storage Device\0";
			case InterfaceClassHub:
				if (device->Descriptor.bcdUSB == 0x210)
					return "USB 2.1 Hub\0";
				else if (device->Descriptor.bcdUSB == 0x200)
					return "USB 2.0 Hub\0";
				else if (device->Descriptor.bcdUSB == 0x110)
					return "USB 1.1 Hub\0";
				else if (device->Descriptor.bcdUSB == 0x100)
					return "USB 1.0 Hub\0";
				else
					return "USB Hub\0";
			case InterfaceClassCdcData:
				return "USB CDC-Data Device\0";
			case InterfaceClassSmartCard:
				return "USB Smart Card\0";
			case InterfaceClassContentSecurity:
				return "USB Content Secuity Device\0";
			case InterfaceClassVideo:
				return "USB Video Device\0";
			case InterfaceClassPersonalHealthcare:
				return "USB Healthcare Device\0";
			case InterfaceClassAudioVideo:
				return "USB AV Device\0";
			case InterfaceClassDiagnosticDevice:
				return "USB Diagnostic Device\0";
			case InterfaceClassWirelessController:
				return "USB Wireless Controller\0";
			case InterfaceClassMiscellaneous:
				return "USB Miscellaneous Device\0";
			case InterfaceClassVendorSpecific:
				return "Vendor Specific\0";
			default:
				return "Generic Device\0";
			}
		}
		else if (device->Descriptor.bDeviceClass == DeviceClassVendorSpecific)
			return "Vendor Specific\0";
		else
			return "Unconfigured Device\0";
	default:
		return "Generic Device\0";
	}
}

/*-UsbShowTree --------------------------------------------------------------
 Shows the USB tree as ascii art using the Printf command. The normal command
 to show from roothub up is  UsbShowTree(UsbGetRootHub(), 1, '+');
 14Mar17 LdB
 --------------------------------------------------------------------------*/
static int TreeLevelInUse[20] = { 0 };
const char* SpeedString[3] = { "High", "Full", "Low" };

void UsbShowTree(struct UsbDevice *root, const int level, const char tee) {
	int maxPacket;
	for (int i = 0; i < level - 1; i++)
		if (TreeLevelInUse[i] == 0) printf("   ");
			else printf(" %c ", '\xB3');							// Draw level lines if in use	
			maxPacket = SizeToNumber(root->Pipe0.MaxSize);			// Max packet size
	printf(" %c-%s id: %i port: %i speed: %s packetsize: %i %s\n", tee,
		UsbGetDescription(root), root->Pipe0.Number, root->ParentHub.PortNumber,
		SpeedString[root->Pipe0.Speed], maxPacket,
		(IsHid(root->Pipe0.Number)) ? "- HID interface" : "");		// Print this entry
	if (IsHub(root->Pipe0.Number)) {
		int lastChild = root->HubPayload->MaxChildren;
		for (int i = 0; i < lastChild; i++) {						// For each child of hub
			char nodetee = '\xC0';									// Preset nodetee to end node ... "L"
			for (int j = i; j < lastChild - 1; j++) {				// Check if any following child node is valid
				if (root->HubPayload->Children[j + 1]) {			// We found a following node in use					
					TreeLevelInUse[level] = 1;						// Set tree level in use flag
					nodetee = (char)0xc3;							// Change the node character to tee looks like this "├"
					break;											// Exit loop j
				};
			}
			if (root->HubPayload->Children[i]) {					// If child valid
				UsbShowTree(root->HubPayload->Children[i],
					level + 1, nodetee);							// Iterate into child but level+1 down of coarse
			}
			TreeLevelInUse[level] = 0;								// Clear level in use flag
		}
	}
}

/*--------------------------------------------------------------------------}
{						 PUBLIC HID INTERFACE ROUTINES						}
{--------------------------------------------------------------------------*/

/*- HIDReadDescriptor ------------------------------------------------------
 Reads the HID descriptor from the given device. The call will error if the
 device is not a HID device, you can always check that by the use of IsHID.
 23Mar17 LdB
 --------------------------------------------------------------------------*/
RESULT HIDReadDescriptor (uint8_t devNumber,						// Device number (address) of the device to read 
						  uint8_t hidIndex,							// Which hid configuration information is requested from
						  uint8_t* Buffer,							// Pointer to a buffer to receive the descriptor
						  uint16_t Length)							// Maxium length of the buffer 
{
	RESULT result;
	struct UsbDevice* device;
	uint32_t transfer = 0;											// Preset transfer to zero
	volatile uint8_t Hi;
	volatile uint8_t Lo;

	if ((Buffer == NULL) || (Length == 0))	return ErrorArgument;	// Check buffer and length is valid
	if ((devNumber == 0) || (devNumber > MaximumDevices))
		return ErrorDeviceNumber;									// Device number not valid
	device = &DeviceTable[devNumber-1];								// Fetch pointer to device number requested
	if (device->PayLoadId == 0) return ErrorDeviceNumber;			// The requested device isn't in use
	if ((device->PayLoadId != HidPayload) || (device->HidPayload == NULL))
		return ErrorNotHID;											// The device requested isn't a HID device
	if (hidIndex > device->HidPayload->MaxHID) return ErrorIndex;	// Invalid HID descriptor index requested
																	// Calculate HID descriptor size
	Hi = *(uint8_t*)&device->HidPayload->Descriptor[hidIndex].LengthHi; // ARM7/8 alignment issue
	Lo = *(uint8_t*)&device->HidPayload->Descriptor[hidIndex].LengthLo; // ARM7 / 8 alignment issue
	uint16_t sizeToRead = (int)Hi << 8 | Lo;						// Total size we need to read

	/* Okay read the HID descriptor */
	result = HCDGetDescriptor(device->Pipe0, USB_DESCRIPTOR_TYPE_HID_REPORT, 0,
		device->HidPayload->HIDInterface[hidIndex],					// Index number of HID index
		Buffer, sizeToRead, 0x81, &transfer, false);				// Read the HID report descriptor 	
	if ((result != OK) || (transfer != sizeToRead)) {				// Read/transfer failed
		LOG("HCD: Fetch HID descriptor %i for device: %i failed.\n",
			device->HidPayload->HIDInterface[hidIndex], 
			device->Pipe0.Number);									// Log the error
		return ErrorDevice;											// No idea what problem is so bail
	}

	// We buffered for DMA alignment .. Now transfer to user pointer
	if (Length < sizeToRead) sizeToRead = Length;					// Insufficient buffer size for descriptor
	return OK;														// Return success
}


/*- HIDReadReport ----------------------------------------------------------
 Reads the HID report from the given device. The call will error if device
 is not a HID device, you can always check that by the use of IsHID.
 23Mar17 LdB
 --------------------------------------------------------------------------*/
RESULT HIDReadReport (uint8_t devNumber,							// Device number (address) of the device to read
					  uint8_t hidIndex,								// Which hid configuration information is requested from
					  uint16_t reportValue,							// Hi byte = enum HidReportType  Lo Byte = Report Index (0 = default) 
					  uint8_t* Buffer,								// Pointer to a buffer to recieve the report
					  uint16_t Length)								// Length of the report
{
	RESULT result;
	struct UsbDevice* device;
	uint32_t transfer = 0;											// Preset transfer to zero
	struct UsbPipeControl pipectrl = {
		.Channel = dwc_get_free_channel(),							// Find first free channel
		.Type = USB_TRANSFER_TYPE_CONTROL,							// This is a control request
		.Direction = USB_DIRECTION_IN,								// In to host as we are getting
	};

	if ((Buffer == NULL) || (Length == 0))	return ErrorArgument;	// Check buffer and length is valid
	if ((devNumber == 0) || (devNumber > MaximumDevices))
		return ErrorDeviceNumber;									// Device number not valid
	device = &DeviceTable[devNumber-1];								// Fetch pointer to device number requested
	if (device->PayLoadId == 0) return ErrorDeviceNumber;			// The requested device isn't in use
	if ((device->PayLoadId != HidPayload) || (device->HidPayload == NULL))
		return ErrorNotHID;											// The device requested isn't a HID device

	result = HCDSumbitControlMessage(
		device->Pipe0,												// Control pipe
		pipectrl,
		Buffer,														// Pass buffer pointer
		Length,														// Read length requested
		&(struct UsbDeviceRequest) {
			.Request = GetReport,									// Get report
			.Type = 0xa1,											// D7 = Device to Host, D5 = Vendor, D0 = Interface = 1010 0001 = 0xA1	
			.Index = device->HidPayload->HIDInterface[hidIndex],	// HID interface
			.Value = reportValue,									// Report value requested
			.Length = Length,
		},
		ControlMessageTimeout,										// The standard timeout for any control message
		&transfer);													// Monitor transfer byte count
	dwc_release_channel(pipectrl.Channel);							// Release the channel
	if (result != OK) return result;								// Return error
	return OK;														// Return success
}


/*- HIDWriteReport ----------------------------------------------------------
 Writes the HID report located in buffer to the given device. This call will
 error if device is not a HID device, you can always check that by the use of
 IsHID.
 23Mar17 LdB
 --------------------------------------------------------------------------*/
RESULT HIDWriteReport (uint8_t devNumber,							// Device number (address) of the device to write report to
					   uint8_t hidIndex,							// Which hid configuration information is writing to
					   uint16_t reportValue,						// Hi byte = enum HidReportType  Lo Byte = Report Index (0 = default) 
					   uint8_t* Buffer,								// Pointer to a buffer containing the report
					   uint16_t Length)								// Length of the report
{
	RESULT result;
	struct UsbDevice* device;
	uint32_t transfer = 0;											// Preset transfer to zero
	struct UsbPipeControl pipectrl = {
		.Channel = dwc_get_free_channel(),							// Find first free channel
		.Type = USB_TRANSFER_TYPE_CONTROL,							// This is a control request
		.Direction = USB_DIRECTION_OUT,								// Out to device we are setting
	};
	if ((Buffer == NULL) || (Length == 0))	return ErrorArgument;	// Check buffer and length is valid
	if ((devNumber == 0) || (devNumber > MaximumDevices))
		return ErrorDeviceNumber;									// Device number not valid
	device = &DeviceTable[devNumber-1];								// Fetch pointer to device number requested
	if (device->PayLoadId == 0) return ErrorDeviceNumber;			// The requested device isn't in use
	if ((device->PayLoadId != HidPayload) || (device->HidPayload == NULL))
		return ErrorNotHID;											// The device requested isn't a HID device
	result = HCDSumbitControlMessage(
		device->Pipe0,												// Control pipe
		pipectrl,
		Buffer,														// Transfer buffer pointer
		Length,														// Write length requested
		&(struct UsbDeviceRequest) {
			.Request = SetReport,									// Set report
			.Type = 0x21,											// D7 = Host to Device  D5 = Vendor, D0 = Interface = 0010 0001 = 0x21	
			.Index = device->HidPayload->HIDInterface[hidIndex],	// HID interface
			.Value = reportValue,									// Report value requested
			.Length = Length,										// Length of report
		},
		ControlMessageTimeout,										// The standard timeout for any control message
		&transfer);													// Monitor transfer byte count
	dwc_release_channel(pipectrl.Channel);							// Release the channel
	if (result != OK) return result;								// Return error
	if (transfer != Length) return ErrorGeneral;					// Device didn't accept all the data
	return OK;														// Return success
}

/*- HIDSetProtocol ----------------------------------------------------------
Many USB HID devices support multiple low level protocols. For example most
mice and keyboards have a BIOS Boot mode protocol that makes them look like
an old DOS keyboard. They also have another protocol which is more advanced.
This call enables the switch between protocols. What protocols are available
and what interface is retrieved and parsed from Descriptors from the device.
23Mar17 LdB
--------------------------------------------------------------------------*/
RESULT HIDSetProtocol (uint8_t devNumber,							// Device number (address) of the device
					   uint8_t interface,							// Interface number to change protocol on
					   uint16_t protocol)							// The protocol number request
{
	RESULT result;
	struct UsbDevice* device;
	struct UsbPipeControl pipectrl = {
		.Channel = dwc_get_free_channel(),							// Find first free channel
		.Type = USB_TRANSFER_TYPE_CONTROL,							// This is a control request
		.Direction = USB_DIRECTION_OUT,								// Out to device we are setting
	};
	if ((devNumber == 0) || (devNumber > MaximumDevices))
		return ErrorDeviceNumber;		// Device number not valid
	device = &DeviceTable[devNumber-1];								// Fetch pointer to device number requested
	if (device->PayLoadId == 0) return ErrorDeviceNumber;			// The requested device isn't in use
	if ((device->PayLoadId != HidPayload) || (device->HidPayload == NULL))
		return ErrorNotHID;											// The device requested isn't a HID device

	result = HCDSumbitControlMessage(
		device->Pipe0,												// Use the control pipe
		pipectrl,
		NULL,														// No buffer for command
		0,															// No buffer length because of above
		&(struct UsbDeviceRequest) {
			.Request = SetProtocol,									// Set protocol request
			.Type = 0x21,											// D7 = Host to Device  D5 = Vendor D0 = Interface = 0010 0001 = 0x21	
			.Index = interface,										// Interface
			.Value = protocol,										// Protocol
			.Length = 0,											// No data for command
		},
		ControlMessageTimeout,										// Standard control message timeout
		NULL);														// No data so can ignore transfer bytes
	dwc_release_channel(pipectrl.Channel);							// Release the channel
	return result;
}
