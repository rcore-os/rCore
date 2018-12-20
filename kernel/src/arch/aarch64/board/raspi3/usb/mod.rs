mod c_structure;
mod c_structure_usb_2_0;
mod c_structure_usb_1_11;
mod c_api;

use self::c_structure::*;
use self::c_api::*;

pub fn init() {
    unsafe {
        check_size();
        UsbInitialise();
    }
}

pub fn get_root_hub() -> *mut UsbDevice {
    unsafe {
        UsbGetRootHub()
    }
}

pub fn UsbShowTree(root:*mut UsbDevice, level:i32 , tee:char) {
    let mut maxPacket:i32 =0;
    let mut TreeLevelInUse:[i32;20] = [0;20];
    for i in 0..(level-2)as usize {
        if TreeLevelInUse[i] == 0 {
            print!("   ")
        } else {
            print!(" | ")                    // Draw level lines if in use
        }
    }
/*    maxPacket = SizeToNumber(root->Pipe0.MaxSize);                    // Max packet size
    printf(" %c-%s id: %i port: %i speed: %s packetsize: %i %s\n", tee,
        UsbGetDescription(root), root->Pipe0.Number, root->ParentHub.PortNumber,
        SpeedString[root->Pipe0.Speed], maxPacket,
        (IsHid(root->Pipe0.Number)) ? "- HID interface" : "");        // Print this entry
    if (IsHub(root->Pipe0.Number)) {
        int lastChild = root->HubPayload->MaxChildren;
        for (int i = 0; i < lastChild; i++) {                        // For each child of hub
            char nodetee = '\xC0';                                    // Preset nodetee to end node ... "L"
            for (int j = i; j < lastChild - 1; j++) {                // Check if any following child node is valid
                if (root->HubPayload->Children[j + 1]) {            // We found a following node in use                    
                    TreeLevelInUse[level] = 1;                        // Set tree level in use flag
                    nodetee = (char)0xc3;                            // Change the node character to tee looks like this "├"
                    break;                                            // Exit loop j
                };
            }
            if (root->HubPayload->Children[i]) {                    // If child valid
                UsbShowTree(root->HubPayload->Children[i],
                    level + 1, nodetee);                            // Iterate into child but level+1 down of coarse
            }
            TreeLevelInUse[level] = 0;                                // Clear level in use flag
        }
    }*/
}