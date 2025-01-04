# SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
# SPDX-License-Identifier: GPL-3.0-only

self:
{ lib, pkgs, ... }: {
  name = "zonegen";
  nodes.machine = { config, pkgs, ... }: {
    imports = [
      self.inputs.dyndnsd.nixosModules.dyndnsd
    ];

    systemd.services.create-bind-dyn-dir = {
      description = "Service that creates the directory for dynamic DNS zone files";
      before = [ "dyndnsd.service" "bind.service" ];
      requiredBy = [ "dyndnsd.service" "bind.service" ];
      serviceConfig = {
        Type = "oneshot";
        Group = "zonegen";
      };
      startLimitBurst = 1;
      script = ''
        mkdir -p '/var/lib/bind/zones/dyn/'
        chmod 775 '/var/lib/bind/zones/dyn/'
        chgrp zonegen '/var/lib/bind/zones/dyn/'

        # Create an initial file for BIND to read
        (set -o noclobber;>'/var/lib/bind/zones/dyn/example.org.zone'||true) &>/dev/null
      '';
    };

    # Check if we have write permission on the file itself,
    # and replace the file with a writable version if we don't.
    # This is unfortunately not atomic.
    # This could be avoided if the tempfile-fast rust crate allowed ignoring the ownership of the old file.
    systemd.services.dyndnsd.preStart = ''
      if ! [ -w '/var/lib/bind/zones/dyn/example.org.zone' ]; then
        # Copy the file, changing ownership
        cp '/var/lib/bind/zones/dyn/example.org.zone' '/var/lib/bind/zones/dyn/example.org.zone.tmp'
        # Replace the old file
        mv '/var/lib/bind/zones/dyn/example.org.zone.tmp' '/var/lib/bind/zones/dyn/example.org.zone'
      fi
    '';

    systemd.tmpfiles.settings."bind"."/var/lib/bind/zones/example.org/".d = {
      user = "named";
      group = "named";
    };

    systemd.services.bind.preStart = let
      zoneFile = pkgs.writeText "root.zone" ''
        $ORIGIN example.org.
        $TTL 3600
        @ IN SOA ns.example.org. admin.example.org. ( 1 3h 1h 1w 1d )
        @ IN NS ns.example.org.

        ns IN A    127.0.0.1
        ns IN AAAA ::1

        1.0.0.127.in-addr.arpa IN PTR ns.example.org.
        1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.ip6.arpa IN PTR ns.example.org.

        $INCLUDE /var/lib/bind/zones/dyn/example.org.zone
      '';
    in ''
      cp '${zoneFile}' '/var/lib/bind/zones/example.org/root.zone'
    '';

    services.bind = {
      enable = true;
      forward = "only";
      forwarders = [];
      extraOptions = ''
        empty-zones-enable no;
      '';
      zones = {
        "example.org" = {
          file = "/var/lib/bind/zones/example.org/root.zone";
          master = true;
        };
      };
    };

    services.dyndnsd = {
      enable = true;
      useZonegen = true;
      settings = {
        users = {
          alice = {
            hash = "$argon2id$v=19$m=65536,t=3,p=1$ZFRHDlJOQ3UNQRN7em14R08FIRE$0SqSQRj45ZBz1MfCPq9DVMWt7VSl96m7XtW6maIcUB0";
            domains = {
              "example.org" = {
                ttl = 60;
                ipv6prefixlen = 48;
                ipv6suffix = "0:0:0:1::5";
              };
              "test.example.org" = {
                ttl = 300;
                ipv6prefixlen = 48;
                ipv6suffix = "0:0:0:1::6";
              };
            };
          };
          bob = {
            hash = "$argon2id$v=19$m=65536,t=3,p=1$cVV0AzdTOAMwAzhmRAM6Yl8QDm4$SC2GeQjXWT+gnYHp5MYFr4m2OxRevsohluqv3EPVgSY";
            domains = {
              "bob.example.org" = {
                ttl = 300;
                ipv6prefixlen = 48;
                ipv6suffix = "0:0:0:1::7";
              };
            };
          };
        };
      };
    };
    environment.systemPackages = [
      pkgs.dig.dnsutils # Make the `dig` command available in the test script
      pkgs.dig.out      # Make the `rndc` command available in the test script
    ];
  };
  testScript = ''
    def query(
        query: str,
        query_type: str,
        expected: str,
    ):
        """
        Execute a single query and and compare the result with expectation
        """
        out = machine.succeed(
            f"dig {query} {query_type} +short"
        ).strip()
        machine.log(f"DNS server replied with {out}")
        assert expected == out, f"Expected `{expected}` but got `{out}`"

    start_all()
    machine.wait_for_unit("dyndnsd.service")
    machine.wait_for_unit("bind.service")
    machine.succeed("curl --fail-with-body -v 'http://[::1]:9841/update?user=alice&pass=123456&ipv4=2.3.4.5&ipv6=2:3:4:5:6:7:8:9'")
    machine.succeed("curl --fail-with-body -v 'http://[::1]:9841/update?user=bob&pass=234567&ipv4=3.4.5.6&ipv6=3:4:5:6:7:8:9:0'")
    # Tell BIND to reload the zone file (use https://github.com/Luflosi/zonewatch in a real deployment, this also increments the serial number)
    machine.succeed("rndc reload example.org")
    query("example.org", "A", "2.3.4.5")
    query("example.org", "AAAA", "2:3:4:1::5")
    query("test.example.org", "A", "2.3.4.5")
    query("test.example.org", "AAAA", "2:3:4:1::6")
    query("bob.example.org", "A", "3.4.5.6")
    query("bob.example.org", "AAAA", "3:4:5:1::7")
  '';
}
