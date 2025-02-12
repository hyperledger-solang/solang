setup
=====
A server config utility for nodejs
Change: hostname, network interfaces, hosts and date/time

# Features

- Set your network configuration. supports wireless adapters
- Change your hostname
- Set your hosts file (local dns)
- Modify server date/time and BIOS update
- Only works in linux :)

You need to install wpasupplicant for wireless options


# Install
```bash
npm install setup
```

# API

### Networking
- setup.network.config(config)  // Creates/returns a new network config file
- setup.network.save(config, outFile) 	  // Saves the configuration
- setup.network.restart() 	  // Restart network interfaces


### Hostname
- setup.hosts.save(hostname, outFile)


### Hosts (dns)
- setup.hosts.config(hosts)
- setup.hosts.save(config, outFile)


### Date/Time
- setup.clock.set(time) // Set date/time and sync BIOS clock


# Examples

### Set network interfaces

This will set your wlan0 card to connect at boot, use dhcp for ip settings, e connect to the SSID 'myWirelessName'.
Your ethernet card will have a static ip.

```js
var setup = require('setup')();

var config = setup.network.config({

	wlan0: {
		auto: true, // start at Boot
		dhcp: true, // Use DHCP
		wireless: {
			ssid: 'myWirelessName', // Wireless SSID
			psk: 'mySuperPassword', // Password
		}
	},
	eth0: {
		auto: true,
		ipv4: {
			address: '192.168.1.20',
			netmask: '255.255.255.0',
			gateway: '192.168.1.1',
			dns: '8.8.8.8'
		}
	}
});

setup.network.save(config);
```


### Change Hostname
```js
setup.hostname.save('nodejs.example.com');
```

### Change hosts
```js
var hosts = setup.hosts.config({ 
	'10.0.0.1':'server1.example.com', 
	'10.0.0.2':'server2.example.com'
});

setup.hosts.save(hosts);
```


