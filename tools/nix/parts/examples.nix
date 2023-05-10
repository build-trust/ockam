{
  perSystem = {pkgs, ...}: {
    devShells.influx = pkgs.mkShell {
      packages = with pkgs; [influxdb2 influxdb2-cli telegraf];
    };
  };
}
