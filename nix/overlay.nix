# SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
# SPDX-License-Identifier: GPL-3.0-only

builder: crane: fenix:
final: prev: let
  system = prev.stdenv.hostPlatform.system;
  builder' = builder {
    inherit crane fenix system;
    pkgs = final;
  };
in {
  zonegen = builder'.zonegen;
}
