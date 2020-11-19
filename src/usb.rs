use anyhow::Result;
use usbip::{EndpointAttributes, UsbDevice, UsbEndpoint, UsbHostHandler, UsbInterface, UsbInterfaceHandler, UsbIpServer, server};
use std::sync::{Arc, Mutex};

pub async fn test(wanted_id: &str) -> Result<()> {
    let mut devices = vec![];
    let mut i = 0;
    if let Ok(list) = rusb::devices() {
        for dev in list.iter() {
            let desc = dev.device_descriptor().unwrap();
            let id = format!("{:x}:{:x}", desc.vendor_id(), desc.product_id());
            // if id != wanted_id {
            //     println!("{}", id);
            //     continue;
            // }

            let handle = Arc::new(Mutex::new(match dev.open() {
                Ok(h) => h,
                Err(_) => continue,
            }));

            let cfg = dev.active_config_descriptor().unwrap();
            let mut interfaces = vec![];
            handle
                .lock()
                .unwrap()
                .set_auto_detach_kernel_driver(true)
                .ok();
            for intf in cfg.interfaces() {
                // ignore alternate settings
                let intf_desc = intf.descriptors().next().unwrap();
                if let Err(_) = handle
                    .lock()
                    .unwrap()
                    .claim_interface(intf_desc.interface_number())
                {
                    break;
                }

                let mut endpoints = vec![];

                for ep_desc in intf_desc.endpoint_descriptors() {
                    endpoints.push(UsbEndpoint {
                        address: ep_desc.address(),
                        attributes: ep_desc.transfer_type() as u8,
                        max_packet_size: ep_desc.max_packet_size(),
                        interval: ep_desc.interval(),
                    });
                }

                let handler =
                    Arc::new(Mutex::new(Box::new(UsbHostHandler::new(handle.clone()))
                        as Box<dyn UsbInterfaceHandler + Send>));
                interfaces.push(UsbInterface {
                    interface_class: intf_desc.class_code(),
                    interface_subclass: intf_desc.sub_class_code(),
                    interface_protocol: intf_desc.protocol_code(),
                    endpoints,
                    string_interface: intf_desc.description_string_index().unwrap_or(0),
                    class_specific_descriptor: Vec::from(intf_desc.extra().unwrap_or(&[])),
                    handler,
                });
            }
        
            // let mut string_manufacturer: Option<String> = None;
            // let mut string_product: Option<String> = None;
            // let mut string_serial: Option<String> = None;
            // // set strings
            // if let Some(index) = desc.manufacturer_string_index() {
            //     string_manufacturer = Some(handle
            //         .lock()
            //         .unwrap()
            //         .read_string_descriptor_ascii(index)
            //         .unwrap()
            //     );
            // }
            // if let Some(index) = desc.product_string_index() {
            //     string_product = Some(
            //         handle
            //             .lock()
            //             .unwrap()
            //             .read_string_descriptor_ascii(index)
            //             .unwrap()
            //     );
            // }
            // if let Some(index) = desc.serial_number_string_index() {
            //     string_serial = Some(
            //         handle
            //             .lock()
            //             .unwrap()
            //             .read_string_descriptor_ascii(index)P
            //             .unwrap(),
            //     );
            // }

            if interfaces.len() > 0 {
                let mut device = UsbDevice::new(i);
                for interf in interfaces {
                    device = device.with_interface(interf.interface_class, interf.interface_subclass, interf.interface_protocol, &id, interf.endpoints, interf.handler);
                }
                device.vendor_id = desc.vendor_id();
                device.product_id = desc.product_id();
                device.device_class = desc.class_code();
                device.device_subclass = desc.sub_class_code();
                device.device_protocol = desc.protocol_code();
                device.configuration_value = cfg.number();
                device.num_configurations = desc.num_configurations();
    
                let mut string_product: Option<String> = None;
                if let Some(index) = desc.product_string_index() {
                    string_product = Some(
                        handle
                            .lock()
                            .unwrap()
                            .read_string_descriptor_ascii(index)
                            .unwrap()
                    );
                }
                devices.push(device);

                log::trace!("Added device {} {} {:?}", i, id, string_product);
                i += 1;
            } else {
                log::trace!("Ignored device {}", id);
            }
        }
    }
    log::trace!("start server");
    server("0.0.0.0:12345".parse().unwrap(), UsbIpServer::new_simulated(devices)).await;
    Ok(())
}
