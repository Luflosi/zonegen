# SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
# SPDX-License-Identifier: GPL-3.0-only

self:
{ lib, pkgs, ... }: {
  name = "zonegen";
  nodes.machine = { config, pkgs, ... }: {
    imports = [
      self.inputs.dyndnsd.nixosModules.dyndnsd
    ];

    users.groups.zonegen = {};
    systemd.services.create-bind-dyn-dir = {
      description = "Service that creates the directory for dynamic DNS zone files";
      before = [ "dyndnsd.service" "bind.service" ];
      wantedBy = [ "dyndnsd.service" "bind.service" ];
      startLimitBurst = 1;
      script = ''
        mkdir -p '/var/lib/bind/dyn/'
        chgrp zonegen '/var/lib/bind/dyn/'
        chmod 775 '/var/lib/bind/dyn/'
        if ! [ -f "/var/lib/bind/dyn/example.org.zone" ]; then
          # Create an initial file for BIND to read
          touch '/var/lib/bind/dyn/example.org.zone'
        fi
      '';
    };

    # Check if we have write permission on the file itself,
    # and replace the file with a writable version if we don't.
    # This is unfortunately not atomic.
    # This could be avoided if the tempfile-fast rust crate allowed ignoring the ownership of the old file.
    systemd.services.dyndnsd.preStart = ''
      if ! [ -w '/var/lib/bind/dyn/example.org.zone' ]; then
        # Copy the file, changing ownership
        cp '/var/lib/bind/dyn/example.org.zone' '/var/lib/bind/dyn/example.org.zone.tmp'
        # Replace the old file
        mv '/var/lib/bind/dyn/example.org.zone.tmp' '/var/lib/bind/dyn/example.org.zone'
      fi
    '';

    systemd.services.dyndnsd.serviceConfig = {
      SupplementaryGroups = [ "zonegen" ];
      ReadWritePaths = [ "/var/lib/bind/dyn/" ];

      # The tempfile-fast rust crate tries to keep the old permissions, so we need to allow this class of system calls
      SystemCallFilter = [ "@chown" ];
      UMask = "0022"; # Allow all processes (including BIND) to read the zone files (and database)
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

        $INCLUDE /var/lib/bind/dyn/example.org.zone
      '';
    in ''
      mkdir -p '/var/lib/bind/zones/example.org/'
      chown -R named '/var/lib/bind/zones/example.org/'
      cp '${zoneFile}' '/var/lib/bind/zones/example.org/root.zone'
      chown named '/var/lib/bind/zones/example.org/root.zone'
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
      settings = {
        update_program = {
          bin = "${pkgs.zonegen}/bin/zonegen";
          args = [ "--dir" "/var/lib/bind/dyn/" ];
          initial_stdin = "drop\n";
          stdin_per_zone_update = "send\n";
          final_stdin = "quit\n";
          ipv4.stdin = "update add {domain}. {ttl} IN A {ipv4}\n";
          ipv6.stdin = "update add {domain}. {ttl} IN AAAA {ipv6}\n";
        };
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

    machine.start()
    machine.wait_for_unit("dyndnsd.service")
    machine.wait_for_unit("bind.service")
    machine.succeed("curl --fail-with-body -v 'http://[::1]:9841/update?user=alice&pass=123456&ipv4=2.3.4.5&ipv6=2:3:4:5:6:7:8:9'")
    # TODO: update serial number in zone file (I plan to do this with an external daemon)
    machine.succeed("rndc reload example.org")
    query("example.org", "A", "2.3.4.5")
    query("example.org", "AAAA", "2:3:4:1::5")
    query("test.example.org", "A", "2.3.4.5")
    query("test.example.org", "AAAA", "2:3:4:1::6")
  '';
}