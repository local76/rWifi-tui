#![allow(dead_code, non_snake_case)]
use std::fmt::{Debug, Formatter};

#[cfg(not(windows))]
pub mod windows_sys {
    pub mod core {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct GUID {
            pub data1: u32,
            pub data2: u16,
            pub data3: u16,
            pub data4: [u8; 8],
        }
    }
}

#[derive(Clone)]
pub struct WlanNetwork {
    pub ssid: String,
    pub signal_quality: u32,
    pub is_connected: bool,
    pub has_profile: bool,
    pub security_enabled: bool,
    pub auth_algorithm: String,
    pub cipher_algorithm: String,
    pub interface_guid: windows_sys::core::GUID,
}

impl std::fmt::Debug for WlanNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WlanNetwork")
            .field("ssid", &self.ssid)
            .field("signal_quality", &self.signal_quality)
            .field("is_connected", &self.is_connected)
            .field("has_profile", &self.has_profile)
            .field("security_enabled", &self.security_enabled)
            .field("auth_algorithm", &self.auth_algorithm)
            .field("cipher_algorithm", &self.cipher_algorithm)
            .field("interface_guid", &format!(
                "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                self.interface_guid.data1,
                self.interface_guid.data2,
                self.interface_guid.data3,
                self.interface_guid.data4[0],
                self.interface_guid.data4[1],
                self.interface_guid.data4[2],
                self.interface_guid.data4[3],
                self.interface_guid.data4[4],
                self.interface_guid.data4[5],
                self.interface_guid.data4[6],
                self.interface_guid.data4[7],
            ))
            .finish()
    }
}

impl PartialEq for WlanNetwork {
    fn eq(&self, other: &Self) -> bool {
        self.ssid == other.ssid
            && self.signal_quality == other.signal_quality
            && self.is_connected == other.is_connected
            && self.has_profile == other.has_profile
            && self.security_enabled == other.security_enabled
            && self.auth_algorithm == other.auth_algorithm
            && self.cipher_algorithm == other.cipher_algorithm
            && self.interface_guid.data1 == other.interface_guid.data1
            && self.interface_guid.data2 == other.interface_guid.data2
            && self.interface_guid.data3 == other.interface_guid.data3
            && self.interface_guid.data4 == other.interface_guid.data4
    }
}

impl Eq for WlanNetwork {}

pub struct RadioState {
    pub software_on: bool,
    pub hardware_on: bool,
}

/// Helper to truncate a string to a max length and append "..." if exceeded.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let mut truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        truncated.push_str("...");
        truncated
    } else {
        s.to_string()
    }
}

// ==========================================
// WINDOWS BACKEND IMPLEMENTATION
// ==========================================
#[cfg(windows)]
pub fn query_wifi_networks(force_scan: bool) -> Result<Vec<WlanNetwork>, u32> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;

    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(res);
    }

    let mut interface_list = std::ptr::null_mut();
    let res = unsafe { WlanEnumInterfaces(handle, std::ptr::null(), &mut interface_list) };
    if res != ERROR_SUCCESS {
        unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };
        return Err(res);
    }

    let interface_list_ref = unsafe { &*interface_list };

    if force_scan {
        for i in 0..interface_list_ref.dwNumberOfItems as usize {
            let interface_info = unsafe { *interface_list_ref.InterfaceInfo.as_ptr().add(i) };
            unsafe {
                WlanScan(
                    handle,
                    &interface_info.InterfaceGuid,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                );
            }
        }
        // Sleep to allow the hardware to complete the channel sweeps
        std::thread::sleep(std::time::Duration::from_millis(1200));
    }

    let mut networks = Vec::new();
    
    for i in 0..interface_list_ref.dwNumberOfItems as usize {
        let interface_info = unsafe { *interface_list_ref.InterfaceInfo.as_ptr().add(i) };
        
        let mut network_list = std::ptr::null_mut();
        let res = unsafe {
            WlanGetAvailableNetworkList(
                handle,
                &interface_info.InterfaceGuid,
                0,
                std::ptr::null(),
                &mut network_list,
            )
        };
        
        if res == ERROR_SUCCESS {
            let network_list_ref = unsafe { &*network_list };
            for j in 0..network_list_ref.dwNumberOfItems as usize {
                let net = unsafe { *network_list_ref.Network.as_ptr().add(j) };
                let ssid_len = net.dot11Ssid.uSSIDLength as usize;
                
                // Convert SSID bytes to string safely
                let ssid = String::from_utf8_lossy(&net.dot11Ssid.ucSSID[..ssid_len]).into_owned();
                
                // Skip empty SSID hidden networks for now
                if ssid.is_empty() {
                    continue;
                }

                // Check if this SSID is already in our list (avoid duplicates if detected on multiple interfaces)
                if networks.iter().any(|n: &WlanNetwork| n.ssid == ssid && n.is_connected) {
                    continue;
                }

                let is_connected = (net.dwFlags & WLAN_AVAILABLE_NETWORK_CONNECTED) != 0;
                let has_profile = (net.dwFlags & WLAN_AVAILABLE_NETWORK_HAS_PROFILE) != 0;
                
                let auth_algo = match net.dot11DefaultAuthAlgorithm {
                    DOT11_AUTH_ALGO_80211_OPEN => "Open",
                    DOT11_AUTH_ALGO_80211_SHARED_KEY => "Shared Key",
                    DOT11_AUTH_ALGO_WPA => "WPA-Enterprise",
                    DOT11_AUTH_ALGO_WPA_PSK => "WPA-Personal",
                    DOT11_AUTH_ALGO_RSNA => "WPA2-Enterprise",
                    DOT11_AUTH_ALGO_RSNA_PSK => "WPA2-Personal",
                    10 => "WPA3-Personal",
                    11 => "WPA3-Enterprise",
                    _ => "Unknown",
                }.to_string();

                let cipher_algo = match net.dot11DefaultCipherAlgorithm {
                    DOT11_CIPHER_ALGO_NONE => "None",
                    DOT11_CIPHER_ALGO_WEP => "WEP",
                    DOT11_CIPHER_ALGO_TKIP => "TKIP",
                    DOT11_CIPHER_ALGO_CCMP => "AES-CCMP",
                    100 => "GCMP-256",
                    _ => "Unknown",
                }.to_string();

                networks.push(WlanNetwork {
                    ssid,
                    signal_quality: net.wlanSignalQuality,
                    is_connected,
                    has_profile,
                    security_enabled: net.bSecurityEnabled != 0,
                    auth_algorithm: auth_algo,
                    cipher_algorithm: cipher_algo,
                    interface_guid: interface_info.InterfaceGuid,
                });
            }
            unsafe { WlanFreeMemory(network_list as *mut _) };
        }
    }

    unsafe {
        WlanFreeMemory(interface_list as *mut _);
        WlanCloseHandle(handle, std::ptr::null_mut());
    }

    // Sort networks so that connected is first, then by signal quality
    networks.sort_by(|a, b| {
        if a.is_connected && !b.is_connected {
            std::cmp::Ordering::Less
        } else if !a.is_connected && b.is_connected {
            std::cmp::Ordering::Greater
        } else {
            b.signal_quality.cmp(&a.signal_quality)
        }
    });

    Ok(networks)
}

#[cfg(windows)]
fn escape_xml(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(windows)]
pub fn connect_to_wifi(ssid: &str, password: Option<&str>, net: &WlanNetwork) -> Result<(), String> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;

    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(format!("WlanOpenHandle failed: {}", res));
    }

    let profile_name = ssid.to_string();

    if !net.has_profile {
        // We need to create an XML profile.
        let xml_profile = if net.security_enabled {
            let pwd = password.ok_or_else(|| "Password required for secured network".to_string())?;
            
            let (xml_auth, xml_cipher) = match net.auth_algorithm.as_str() {
                "WPA2-Personal" => ("WPA2PSK", "AES"),
                "WPA-Personal" => ("WPAPSK", "TKIP"),
                "WPA3-Personal" => ("WPA3SAE", "AES"),
                "Open" if net.security_enabled => ("WEP", "WEP"),
                _ => ("WPA2PSK", "AES"), // default fallback WPA2PSK is standard today
            };

            // Convert SSID to hex string
            let ssid_hex = ssid.as_bytes().iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join("");

            let escaped_ssid = escape_xml(ssid);
            let escaped_pwd = escape_xml(pwd);

            format!(
                r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{}</name>
    <SSIDConfig>
        <SSID>
            <hex>{}</hex>
            <name>{}</name>
        </SSID>
    </SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>manual</connectionMode>
    <MSM>
        <security>
            <authEncryption>
                <authentication>{}</authentication>
                <encryption>{}</encryption>
                <useOneX>false</useOneX>
            </authEncryption>
            <sharedKey>
                <keyType>passPhrase</keyType>
                <protected>false</protected>
                <keyMaterial>{}</keyMaterial>
            </sharedKey>
        </security>
    </MSM>
</WLANProfile>"#,
                escaped_ssid, ssid_hex, escaped_ssid, xml_auth, xml_cipher, escaped_pwd
            )
        } else {
            // Open, unsecured network
            let ssid_hex = ssid.as_bytes().iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join("");
            let escaped_ssid = escape_xml(ssid);
            format!(
                r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{}</name>
    <SSIDConfig>
        <SSID>
            <hex>{}</hex>
            <name>{}</name>
        </SSID>
    </SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>manual</connectionMode>
    <MSM>
        <security>
            <authEncryption>
                <authentication>open</authentication>
                <encryption>none</encryption>
                <useOneX>false</useOneX>
            </authEncryption>
        </security>
    </MSM>
</WLANProfile>"#,
                escaped_ssid, ssid_hex, escaped_ssid
            )
        };

        // Set the profile in Windows
        let xml_w: Vec<u16> = xml_profile.encode_utf16().chain(std::iter::once(0)).collect();
        let mut reason_code = 0;
        let res = unsafe {
            WlanSetProfile(
                handle,
                &net.interface_guid,
                0,
                xml_w.as_ptr(),
                std::ptr::null(),
                1,
                std::ptr::null(),
                &mut reason_code,
            )
        };

        if res != ERROR_SUCCESS {
            unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };
            return Err(format!("WlanSetProfile failed: code {}, reason {}", res, reason_code));
        }
    }

    // Now connect to the profile
    let profile_w: Vec<u16> = profile_name.encode_utf16().chain(std::iter::once(0)).collect();
    
    // We connect using connection mode: wlan_connection_mode_profile
    let mut conn_params: WLAN_CONNECTION_PARAMETERS = unsafe { std::mem::zeroed() };
    conn_params.wlanConnectionMode = wlan_connection_mode_profile;
    conn_params.strProfile = profile_w.as_ptr();
    conn_params.dot11BssType = dot11_BSS_type_infrastructure;

    let res = unsafe {
        WlanConnect(
            handle,
            &net.interface_guid,
            &conn_params,
            std::ptr::null(),
        )
    };

    unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };

    if res != ERROR_SUCCESS {
        return Err(format!("WlanConnect failed: {}", res));
    }

    Ok(())
}

#[cfg(windows)]
pub fn disconnect_wifi(interface_guid: &windows_sys::core::GUID) -> Result<(), String> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;

    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(format!("WlanOpenHandle failed: {}", res));
    }

    let res = unsafe { WlanDisconnect(handle, interface_guid, std::ptr::null()) };
    unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };

    if res != ERROR_SUCCESS {
        return Err(format!("WlanDisconnect failed: {}", res));
    }

    Ok(())
}

#[cfg(windows)]
pub fn delete_wifi_profile(ssid: &str, interface_guid: &windows_sys::core::GUID) -> Result<(), String> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;

    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(format!("WlanOpenHandle failed: {}", res));
    }

    let ssid_w: Vec<u16> = ssid.encode_utf16().chain(std::iter::once(0)).collect();
    let res = unsafe { WlanDeleteProfile(handle, interface_guid, ssid_w.as_ptr(), std::ptr::null()) };
    
    unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };

    if res != ERROR_SUCCESS {
        return Err(format!("WlanDeleteProfile failed: {}", res));
    }

    Ok(())
}

#[cfg(windows)]
pub fn get_first_interface_guid() -> Result<windows_sys::core::GUID, u32> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;

    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(res);
    }

    let mut interface_list = std::ptr::null_mut();
    let res = unsafe { WlanEnumInterfaces(handle, std::ptr::null(), &mut interface_list) };
    if res != ERROR_SUCCESS {
        unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };
        return Err(res);
    }

    let interface_list_ref = unsafe { &*interface_list };
    if interface_list_ref.dwNumberOfItems == 0 {
        unsafe {
            WlanFreeMemory(interface_list as *mut _);
            WlanCloseHandle(handle, std::ptr::null_mut());
        }
        return Err(1168); // ERROR_NOT_FOUND = 1168
    }

    let interface_info = unsafe { *interface_list_ref.InterfaceInfo.as_ptr() };
    let guid = interface_info.InterfaceGuid;

    unsafe {
        WlanFreeMemory(interface_list as *mut _);
        WlanCloseHandle(handle, std::ptr::null_mut());
    }

    Ok(guid)
}

#[cfg(windows)]
pub fn query_radio_state(interface_guid: &windows_sys::core::GUID) -> Result<RadioState, u32> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;

    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(res);
    }

    let mut data_size = 0;
    let mut data_ptr = std::ptr::null_mut();
    let mut opcode_value_type = wlan_opcode_value_type_query_only;

    let res = unsafe {
        WlanQueryInterface(
            handle,
            interface_guid,
            wlan_intf_opcode_radio_state,
            std::ptr::null(),
            &mut data_size,
            &mut data_ptr,
            &mut opcode_value_type,
        )
    };

    if res != ERROR_SUCCESS {
        unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };
        return Err(res);
    }

    let radio_state_ref = unsafe { &*(data_ptr as *const WLAN_RADIO_STATE) };
    
    let mut software_on = true;
    let mut hardware_on = true;

    if radio_state_ref.dwNumberOfPhys > 0 {
        let phy_state = radio_state_ref.PhyRadioState[0];
        software_on = phy_state.dot11SoftwareRadioState == dot11_radio_state_on;
        hardware_on = phy_state.dot11HardwareRadioState == dot11_radio_state_on;
    }

    unsafe {
        WlanFreeMemory(data_ptr);
        WlanCloseHandle(handle, std::ptr::null_mut());
    }

    Ok(RadioState { software_on, hardware_on })
}

#[cfg(windows)]
pub fn set_radio_state(interface_guid: &windows_sys::core::GUID, software_on: bool) -> Result<(), u32> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;

    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(res);
    }

    let mut radio_state: WLAN_RADIO_STATE = unsafe { std::mem::zeroed() };
    radio_state.dwNumberOfPhys = 1;
    radio_state.PhyRadioState[0].dwPhyIndex = 0;
    radio_state.PhyRadioState[0].dot11SoftwareRadioState = if software_on {
        dot11_radio_state_on
    } else {
        dot11_radio_state_off
    };

    let res = unsafe {
        WlanSetInterface(
            handle,
            interface_guid,
            wlan_intf_opcode_radio_state,
            std::mem::size_of::<WLAN_RADIO_STATE>() as u32,
            &radio_state as *const _ as *const _,
            std::ptr::null(),
        )
    };

    unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };

    if res != ERROR_SUCCESS {
        return Err(res);
    }

    Ok(())
}

#[cfg(windows)]
pub fn query_saved_password(ssid: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("netsh")
        .args(["wlan", "show", "profile", &format!("name={}", ssid), "key=clear"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .ok()?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.contains("Key Content") || trimmed.contains("Contenido de la clave") || trimmed.contains("Schlüsselinhalt") || trimmed.contains("Contenu de la") {
                if let Some(pos) = trimmed.find(':') {
                    return Some(trimmed[pos + 1..].trim().to_string());
                }
            }
        }
    }
    None
}

#[cfg(windows)]
pub fn query_saved_profiles() -> Result<Vec<(String, windows_sys::core::GUID)>, u32> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    
    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(res);
    }
    
    let mut interface_list = std::ptr::null_mut();
    let res = unsafe { WlanEnumInterfaces(handle, std::ptr::null(), &mut interface_list) };
    if res != ERROR_SUCCESS {
        unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };
        return Err(res);
    }
    
    let mut profiles = Vec::new();
    let interface_list_ref = unsafe { &*interface_list };
    
    for i in 0..interface_list_ref.dwNumberOfItems as usize {
        let interface_info = unsafe { *interface_list_ref.InterfaceInfo.as_ptr().add(i) };
        let mut profile_list = std::ptr::null_mut();
        let res = unsafe {
            WlanGetProfileList(
                handle,
                &interface_info.InterfaceGuid,
                std::ptr::null(),
                &mut profile_list,
            )
        };
        
        if res == ERROR_SUCCESS {
            let profile_list_ref = unsafe { &*profile_list };
            for j in 0..profile_list_ref.dwNumberOfItems as usize {
                let profile_info = unsafe { *profile_list_ref.ProfileInfo.as_ptr().add(j) };
                let len = profile_info.strProfileName.iter().position(|&c| c == 0).unwrap_or(profile_info.strProfileName.len());
                if let Ok(name) = String::from_utf16(&profile_info.strProfileName[..len]) {
                    if !name.is_empty() {
                        profiles.push((name, interface_info.InterfaceGuid));
                    }
                }
            }
            unsafe { WlanFreeMemory(profile_list as *mut _) };
        }
    }
    
    unsafe {
        WlanFreeMemory(interface_list as *mut _);
        WlanCloseHandle(handle, std::ptr::null_mut());
    }
    
    Ok(profiles)
}

#[cfg(windows)]
pub fn connect_to_enterprise_wifi(
    ssid: &str,
    username: &str,
    password: &str,
    net: &WlanNetwork,
) -> Result<(), String> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;

    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(format!("WlanOpenHandle failed: {}", res));
    }

    let profile_name = ssid.to_string();

    let xml_auth = "WPA2";
    let xml_cipher = "AES";
    let ssid_hex = ssid.as_bytes().iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join("");
    let escaped_ssid = escape_xml(ssid);

    let profile_xml = format!(
        r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{}</name>
    <SSIDConfig>
        <SSID>
            <hex>{}</hex>
            <name>{}</name>
        </SSID>
    </SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>manual</connectionMode>
    <MSM>
        <security>
            <authEncryption>
                <authentication>{}</authentication>
                <encryption>{}</encryption>
                <useOneX>true</useOneX>
            </authEncryption>
            <OneX xmlns="http://www.microsoft.com/networking/OneX/v1">
                <authMode>user</authMode>
                <EAPConfig>
                    <EapHostConfig xmlns="http://www.microsoft.com/networking/EapHostConfig">
                        <EapMethod>
                            <Type xmlns="http://www.microsoft.com/grouping/eapandcert/eapmethodconfig">25</Type>
                            <VendorId xmlns="http://www.microsoft.com/grouping/eapandcert/eapmethodconfig">0</VendorId>
                            <AuthorId xmlns="http://www.microsoft.com/grouping/eapandcert/eapmethodconfig">0</AuthorId>
                            <Type xmlns="http://www.microsoft.com/grouping/eapandcert/eapmethodconfig">0</Type>
                        </EapMethod>
                        <Config xmlns="http://www.microsoft.com/networking/EapHostConfig">
                            <EapType xmlns="http://www.microsoft.com/grouping/eapandcert/eapdeviceconfig">
                                <Type>25</Type>
                                <ConfigP xmlns="http://www.microsoft.com/grouping/eapandcert/eapdeviceconfig">
                                    <peapEapConnectionProperties xmlns="http://www.microsoft.com/provisioning/mspeapconnectionpropertiesv1">
                                        <PerformServerValidation>false</PerformServerValidation>
                                        <AcceptServerName>false</AcceptServerName>
                                        <FastReconnect>true</FastReconnect>
                                        <InnerEapOptional>false</InnerEapOptional>
                                        <Eap xmlns="http://www.microsoft.com/provisioning/baseeapconnectionpropertiesv1">
                                            <Type>26</Type>
                                            <Config xmlns="http://www.microsoft.com/provisioning/baseeapconnectionpropertiesv1">
                                                <EapType xmlns="http://www.microsoft.com/provisioning/mschapv2connectionpropertiesv1">
                                                    <UseWinLogonCredentials>false</UseWinLogonCredentials>
                                                </EapType>
                                            </Config>
                                        </Eap>
                                    </peapEapConnectionProperties>
                                </ConfigP>
                            </EapType>
                        </Config>
                    </EapHostConfig>
                </EAPConfig>
            </OneX>
        </security>
    </MSM>
</WLANProfile>"#,
        escaped_ssid, ssid_hex, escaped_ssid, xml_auth, xml_cipher
    );

    let xml_w: Vec<u16> = profile_xml.encode_utf16().chain(std::iter::once(0)).collect();
    let mut reason_code = 0;
    let res = unsafe {
        WlanSetProfile(
            handle,
            &net.interface_guid,
            0,
            xml_w.as_ptr(),
            std::ptr::null(),
            1,
            std::ptr::null(),
            &mut reason_code,
        )
    };

    if res != ERROR_SUCCESS {
        unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };
        return Err(format!("WlanSetProfile failed: code {}, reason {}", res, reason_code));
    }

    let escaped_user = escape_xml(username);
    let escaped_pwd = escape_xml(password);
    let user_xml = format!(
        r#"<EapHostUserCredentials xmlns="http://www.microsoft.com/networking/EapHostUserCredentials" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <EapMethod>
        <Type xmlns="http://www.microsoft.com/grouping/eapandcert/eapmethodconfig">25</Type>
        <VendorId xmlns="http://www.microsoft.com/grouping/eapandcert/eapmethodconfig">0</VendorId>
        <AuthorId xmlns="http://www.microsoft.com/grouping/eapandcert/eapmethodconfig">0</AuthorId>
        <Type xmlns="http://www.microsoft.com/grouping/eapandcert/eapmethodconfig">0</Type>
    </EapMethod>
    <Credentials xmlns="http://www.microsoft.com/networking/EapHostUserCredentials">
        <EapType xmlns="http://www.microsoft.com/grouping/eapandcert/eapdeviceconfig">
            <Type>25</Type>
            <ConfigP xmlns="http://www.microsoft.com/grouping/eapandcert/eapdeviceconfig">
                <peapUserProperties xmlns="http://www.microsoft.com/provisioning/mspeapuserpropertiesv1">
                    <RoutingIdentity xsi:nil="true"/>
                    <Eap xmlns="http://www.microsoft.com/provisioning/baseeapuserpropertiesv1">
                        <Type>26</Type>
                        <Config xmlns="http://www.microsoft.com/provisioning/baseeapuserpropertiesv1">
                            <EapType xmlns="http://www.microsoft.com/provisioning/mschapv2userpropertiesv1">
                                <Username>{}</Username>
                                <Password>{}</Password>
                                <LogonDomain></LogonDomain>
                            </EapType>
                        </Config>
                    </Eap>
                </peapUserProperties>
            </ConfigP>
        </EapType>
    </Credentials>
</EapHostUserCredentials>"#,
        escaped_user, escaped_pwd
    );

    let user_w: Vec<u16> = user_xml.encode_utf16().chain(std::iter::once(0)).collect();
    let profile_w: Vec<u16> = profile_name.encode_utf16().chain(std::iter::once(0)).collect();

    #[link(name = "wlanapi")]
    unsafe extern "system" {
        fn WlanSetProfileEapXmlUserData(
            hClientHandle: *mut std::ffi::c_void,
            pInterfaceGuid: *const windows_sys::core::GUID,
            strProfileName: *const u16,
            dwFlags: u32,
            strEapXmlUserData: *const u16,
            pReserved: *mut std::ffi::c_void,
        ) -> u32;
    }

    let res = unsafe {
        WlanSetProfileEapXmlUserData(
            handle,
            &net.interface_guid,
            profile_w.as_ptr(),
            0,
            user_w.as_ptr(),
            std::ptr::null_mut(),
        )
    };

    if res != ERROR_SUCCESS {
        unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };
        return Err(format!("WlanSetProfileEapXmlUserData failed: code {}", res));
    }

    let mut conn_params: WLAN_CONNECTION_PARAMETERS = unsafe { std::mem::zeroed() };
    conn_params.wlanConnectionMode = wlan_connection_mode_profile;
    conn_params.strProfile = profile_w.as_ptr();
    conn_params.dot11BssType = dot11_BSS_type_infrastructure;

    let res = unsafe {
        WlanConnect(
            handle,
            &net.interface_guid,
            &conn_params,
            std::ptr::null(),
        )
    };

    unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };

    if res != ERROR_SUCCESS {
        return Err(format!("WlanConnect failed: {}", res));
    }

    Ok(())
}

#[cfg(windows)]
pub fn connect_to_hidden_wifi(
    ssid: &str,
    password: Option<&str>,
    security_enabled: bool,
    auth_algo: &str,
    _cipher_algo: &str,
    interface_guid: &windows_sys::core::GUID,
) -> Result<(), String> {
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;

    let mut negotiated_version = 0;
    let mut handle = std::ptr::null_mut();
    
    let res = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut negotiated_version, &mut handle) };
    if res != ERROR_SUCCESS {
        return Err(format!("WlanOpenHandle failed: {}", res));
    }

    let profile_name = ssid.to_string();

    let xml_profile = if security_enabled {
        let pwd = password.ok_or_else(|| "Password required for secured network".to_string())?;
        
        let (xml_auth, xml_cipher) = match auth_algo {
            "WPA2-Personal" | "WPA2PSK" => ("WPA2PSK", "AES"),
            "WPA-Personal" | "WPAPSK" => ("WPAPSK", "TKIP"),
            "WPA3-Personal" | "WPA3SAE" => ("WPA3SAE", "AES"),
            "Open" if security_enabled => ("WEP", "WEP"),
            _ => ("WPA2PSK", "AES"),
        };

        let ssid_hex = ssid.as_bytes().iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join("");
        let escaped_ssid = escape_xml(ssid);
        let escaped_pwd = escape_xml(pwd);

        format!(
            r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{}</name>
    <SSIDConfig>
        <SSID>
            <hex>{}</hex>
            <name>{}</name>
        </SSID>
        <nonBroadcast>true</nonBroadcast>
    </SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>manual</connectionMode>
    <MSM>
        <security>
            <authEncryption>
                <authentication>{}</authentication>
                <encryption>{}</encryption>
                <useOneX>false</useOneX>
            </authEncryption>
            <sharedKey>
                <keyType>passPhrase</keyType>
                <protected>false</protected>
                <keyMaterial>{}</keyMaterial>
            </sharedKey>
        </security>
    </MSM>
</WLANProfile>"#,
            escaped_ssid, ssid_hex, escaped_ssid, xml_auth, xml_cipher, escaped_pwd
        )
    } else {
        let ssid_hex = ssid.as_bytes().iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join("");
        let escaped_ssid = escape_xml(ssid);
        format!(
            r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{}</name>
    <SSIDConfig>
        <SSID>
            <hex>{}</hex>
            <name>{}</name>
        </SSID>
        <nonBroadcast>true</nonBroadcast>
    </SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>manual</connectionMode>
    <MSM>
        <security>
            <authEncryption>
                <authentication>open</authentication>
                <encryption>none</encryption>
                <useOneX>false</useOneX>
            </authEncryption>
        </security>
    </MSM>
</WLANProfile>"#,
            escaped_ssid, ssid_hex, escaped_ssid
        )
    };

    let xml_w: Vec<u16> = xml_profile.encode_utf16().chain(std::iter::once(0)).collect();
    let mut reason_code = 0;
    let res = unsafe {
        WlanSetProfile(
            handle,
            interface_guid,
            0,
            xml_w.as_ptr(),
            std::ptr::null(),
            1,
            std::ptr::null(),
            &mut reason_code,
        )
    };

    if res != ERROR_SUCCESS {
        unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };
        return Err(format!("WlanSetProfile failed: code {}, reason {}", res, reason_code));
    }

    let profile_w: Vec<u16> = profile_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut conn_params: WLAN_CONNECTION_PARAMETERS = unsafe { std::mem::zeroed() };
    conn_params.wlanConnectionMode = wlan_connection_mode_profile;
    conn_params.strProfile = profile_w.as_ptr();
    conn_params.dot11BssType = dot11_BSS_type_infrastructure;

    let res = unsafe {
        WlanConnect(
            handle,
            interface_guid,
            &conn_params,
            std::ptr::null(),
        )
    };

    unsafe { WlanCloseHandle(handle, std::ptr::null_mut()) };

    if res != ERROR_SUCCESS {
        return Err(format!("WlanConnect failed: {}", res));
    }

    Ok(())
}

// ==========================================
// LINUX BACKEND IMPLEMENTATION (nmcli-based)
// ==========================================
#[cfg(not(windows))]
use std::process::Command;

#[cfg(not(windows))]
fn get_wifi_interface() -> String {
    if let Ok(output) = Command::new("nmcli")
        .args(&["-t", "-f", "DEVICE,TYPE", "device"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && parts[1].trim() == "wifi" {
                return parts[0].trim().to_string();
            }
        }
    }
    "wlan0".to_string()
}

#[cfg(not(windows))]
pub fn query_wifi_networks(force_scan: bool) -> Result<Vec<WlanNetwork>, u32> {
    if force_scan {
        let _ = Command::new("nmcli").args(&["device", "wifi", "rescan"]).status();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let output = Command::new("nmcli")
        .args(&["-t", "-f", "SSID,SIGNAL,ACTIVE,SECURITY", "device", "wifi", "list"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut networks = Vec::new();
            let saved_profiles = query_saved_profiles().unwrap_or_default();
            
            for line in stdout.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                
                let mut parts = Vec::new();
                let mut current = String::new();
                let mut chars = line.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '\\' && chars.peek() == Some(&':') {
                        current.push(':');
                        chars.next();
                    } else if c == ':' {
                        parts.push(current.clone());
                        current.clear();
                    } else {
                        current.push(c);
                    }
                }
                parts.push(current);

                if parts.len() < 4 {
                    continue;
                }

                let ssid = parts[0].clone();
                if ssid.is_empty() {
                    continue;
                }
                let signal_quality = parts[1].parse::<u32>().unwrap_or(0);
                let is_connected = parts[2] == "yes";
                let security_str = parts[3].clone();
                let security_enabled = !security_str.is_empty() && security_str != "--";
                let has_profile = saved_profiles.iter().any(|(name, _)| name == &ssid);

                if let Some(existing) = networks.iter_mut().find(|n: &&mut WlanNetwork| n.ssid == ssid) {
                    if signal_quality > existing.signal_quality {
                        existing.signal_quality = signal_quality;
                        existing.is_connected = is_connected;
                    }
                } else {
                    networks.push(WlanNetwork {
                        ssid,
                        signal_quality,
                        is_connected,
                        has_profile,
                        security_enabled,
                        auth_algorithm: security_str,
                        cipher_algorithm: "Unknown".to_string(),
                        interface_guid: Default::default(),
                    });
                }
            }
            Ok(networks)
        }
        _ => Err(1),
    }
}

#[cfg(not(windows))]
pub fn connect_to_wifi(ssid: &str, password: Option<&str>, _net: &WlanNetwork) -> Result<(), String> {
    let mut args = vec!["device", "wifi", "connect", ssid];
    if let Some(pwd) = password {
        if !pwd.is_empty() {
            args.push("password");
            args.push(pwd);
        }
    }
    
    let output = Command::new("nmcli").args(&args).output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).into_owned())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(windows))]
pub fn disconnect_wifi(_interface_guid: &windows_sys::core::GUID) -> Result<(), String> {
    let iface = get_wifi_interface();
    let output = Command::new("nmcli")
        .args(&["device", "disconnect", &iface])
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).into_owned())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(windows))]
pub fn delete_wifi_profile(ssid: &str, _interface_guid: &windows_sys::core::GUID) -> Result<(), String> {
    let output = Command::new("nmcli")
        .args(&["connection", "delete", ssid])
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).into_owned())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(windows))]
pub fn get_first_interface_guid() -> Result<windows_sys::core::GUID, u32> {
    Ok(Default::default())
}

#[cfg(not(windows))]
pub fn query_radio_state(_interface_guid: &windows_sys::core::GUID) -> Result<RadioState, u32> {
    let output = Command::new("nmcli").args(&["radio", "wifi"]).output();
    match output {
        Ok(out) if out.status.success() => {
            let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let software_on = state == "enabled";
            Ok(RadioState { software_on, hardware_on: true })
        }
        _ => Err(1),
    }
}

#[cfg(not(windows))]
pub fn set_radio_state(_interface_guid: &windows_sys::core::GUID, software_on: bool) -> Result<(), u32> {
    let state = if software_on { "on" } else { "off" };
    let status = Command::new("nmcli")
        .args(&["radio", "wifi", state])
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => Err(1),
    }
}

#[cfg(not(windows))]
pub fn query_saved_password(ssid: &str) -> Option<String> {
    if let Ok(output) = Command::new("nmcli")
        .args(&["-s", "-g", "802-11-wireless-security.psk", "connection", "show", ssid])
        .output()
    {
        if output.status.success() {
            let pwd = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !pwd.is_empty() {
                return Some(pwd);
            }
        }
    }
    None
}

#[cfg(not(windows))]
pub fn query_saved_profiles() -> Result<Vec<(String, windows_sys::core::GUID)>, u32> {
    let output = Command::new("nmcli")
        .args(&["-t", "-f", "NAME,TYPE", "connection", "show"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut profiles = Vec::new();
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 && parts[1].trim() == "802-11-wireless" {
                    profiles.push((parts[0].trim().to_string(), Default::default()));
                }
            }
            Ok(profiles)
        }
        _ => Err(1),
    }
}

#[cfg(not(windows))]
pub fn connect_to_enterprise_wifi(
    ssid: &str,
    username: &str,
    password: &str,
    _net: &WlanNetwork,
) -> Result<(), String> {
    let iface = get_wifi_interface();
    let con_name = ssid;
    let _ = Command::new("nmcli").args(&["connection", "delete", con_name]).status();

    let output = Command::new("nmcli")
        .args(&[
            "connection", "add", "type", "wifi",
            "con-name", con_name,
            "ifname", &iface,
            "ssid", ssid,
            "--",
            "wifi-sec.key-mgmt", "wpa-eap",
            "802-1x.identity", username,
            "802-1x.password", password,
            "802-1x.eap", "peap",
            "802-1x.phase2-auth", "mschapv2"
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let up_output = Command::new("nmcli")
                .args(&["connection", "up", "id", con_name])
                .output();
            match up_output {
                Ok(up_out) if up_out.status.success() => Ok(()),
                Ok(up_out) => Err(String::from_utf8_lossy(&up_out.stderr).into_owned()),
                Err(e) => Err(e.to_string()),
            }
        }
        Ok(out) => Err(String::from_utf8_lossy(&out.stderr).into_owned()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(windows))]
pub fn connect_to_hidden_wifi(
    ssid: &str,
    password: Option<&str>,
    security_enabled: bool,
    _auth_algo: &str,
    _cipher_algo: &str,
    _interface_guid: &windows_sys::core::GUID,
) -> Result<(), String> {
    let mut args = vec!["device", "wifi", "connect", ssid, "hidden", "yes"];
    if security_enabled {
        if let Some(pwd) = password {
            if !pwd.is_empty() {
                args.push("password");
                args.push(pwd);
            }
        }
    }
    
    let output = Command::new("nmcli").args(&args).output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).into_owned())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}
