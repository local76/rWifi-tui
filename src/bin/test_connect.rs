#[path = "../wlan.rs"]
mod wlan;

#[cfg(not(windows))]
use wlan::windows_sys;

fn main() {
    println!("Testing connect_to_wifi with a fake network to check for crashes...");
    let fake_guid = windows_sys::core::GUID {
        data1: 0x12345678,
        data2: 0x9abc,
        data3: 0xdef0,
        data4: [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0],
    };
    
    let fake_net = wlan::WlanNetwork {
        ssid: "FakeSecureWiFi".to_string(),
        signal_quality: 100,
        is_connected: false,
        has_profile: false,
        security_enabled: true,
        auth_algorithm: "WPA2-Personal".to_string(),
        cipher_algorithm: "AES-CCMP".to_string(),
        interface_guid: fake_guid,
    };

    println!("Calling connect_to_wifi with password containing special XML characters...");
    match wlan::connect_to_wifi(&fake_net.ssid, Some("pass<&word"), &fake_net) {
        Ok(_) => println!("Result: Ok"),
        Err(e) => {
            println!("Result: Err: {}", e);
            println!("Testing log_windows_event...");
            rcommon::win32::log_windows_event("rWifiTest", 1, 1002, &format!("Failed to connect to FakeSecureWiFi: {}", e));
            println!("log_windows_event completed successfully");
        }
    }
}
