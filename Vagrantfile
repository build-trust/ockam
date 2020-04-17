Vagrant.configure("2") do |config|

  config.vm.define "builder-debian", primary: true do |config|
    config.vm.box = ENV['OCKAM_DEBIAN_BUILDER_BOX'] || "ockam-network/builder-debian"

    boxes_url = "https://ockam.blob.core.windows.net/boxes"
    sha256 = "a6bda9d44f0f0a908ea1a857c179f74c48e647c5b19de6c43ad0d8b9f63fa0c4"
    config.vm.box_url = ENV['OCKAM_DEBIAN_BUILDER_BOX_URL'] || "#{boxes_url}/builder/debian/#{sha256}.box"
    config.vm.box_download_checksum = sha256
    config.vm.box_download_checksum_type = 'sha256'
    config.vm.box_check_update = false

    config.ssh.insert_key = false # TODO: fix this
    config.ssh.keep_alive = true
    config.vm.provision "shell", privileged: true, inline: <<-SCRIPT
      echo 'export OCKAM_C_BASE=/vagrant/implementations/c' > /etc/profile.d/ockam.sh

      apt-get install  apt-transport-https
      echo "deb http://apt.llvm.org/stretch/ llvm-toolchain-stretch-9 main" >> /etc/apt/sources.list
      wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add -
      apt-get update
      apt-get install -y clang-format-9
    SCRIPT

    config.vm.provider :virtualbox do |vbox, override|
      vbox.name = "builder-debian"
      vbox.linked_clone = true
      vbox.customize ["modifyvm", :id, "--cpuexecutioncap", "50"]
      vbox.memory = ENV['OCKAM_DEBIAN_BUILDER_MEMORY'] || 4096
      vbox.cpus = ENV['OCKAM_DEBIAN_BUILDER_CPUS'] || 2

      override.vm.synced_folder ".", "/vagrant", type: :rsync,
        rsync__exclude: [
          'tools/builder/',
          '.builder',
          'implementations/c/_build',
          'implementations/elixir/_build',
          'implementations/elixir/deps',
          'implementations/elixir/priv/native',
          'implementations/rust/target'
        ]
    end

    config.vm.provider :docker do |docker, override|
      override.vm.box = nil
      override.ssh.insert_key = true
      override.vm.synced_folder ".", "/vagrant", docker_consistency: "cached"

      docker.image = "ockam-builder-debian-base:latest"
      docker.name = "builder-debian"
      docker.remains_running = true
      docker.has_ssh = true
      docker.create_args = ['--cap-add', 'SYS_ADMIN', '--tmpfs', '/tmp:exec', '--tmpfs', '/run', '-v', '/sys/fs/cgroup:/sys/fs/cgroup:ro']
    end
  end

  config.vm.define "builder-macos", primary: true do |config|
    config.vm.box = "ockam-network/builder-macos"

    boxes_url = "https://ockam.blob.core.windows.net/boxes-private"
    sha256 = "355dff4ee9783adae79a1d4778693e3ffd7f765f5caebb4cd74501d59fcf7a77"
    config.vm.box_url = "#{boxes_url}/builder/macos/#{sha256}.box?#{ENV['OCKAM_PRIVATE_BOXES_SHARED_ACCESS_TOKEN']}"
    config.vm.box_download_checksum = sha256
    config.vm.box_download_checksum_type = 'sha256'
    config.vm.box_check_update = false

    config.ssh.keep_alive = true
    config.vm.provision "shell", privileged: true, inline: <<-SCRIPT
      echo -e "\nClientAliveInterval 5\nClientAliveCountMax 1000\n" >> /etc/ssh/sshd_config
      launchctl kickstart -k system/com.openssh.sshd
    SCRIPT

    config.vm.provider :virtualbox do |vbox|
      vbox.name = "builder-macos"
      vbox.linked_clone = true
      vbox.customize ["modifyvm", :id, "--cpuexecutioncap", "50"]
      vbox.memory = ENV['OCKAM_MACOS_BUILDER_MEMORY'] || 4096
      vbox.cpus = ENV['OCKAM_MACOS_BUILDER_CPUS'] || 2
    end

    config.vm.synced_folder ".", "/vagrant", type: :rsync,
      rsync__exclude: [
        'tools/builder/',
        '.builder',
        'implementations/c/_build',
        'implementations/elixir/_build',
        'implementations/elixir/priv/native',
        'implementations/rust/target'
      ]
  end
end
