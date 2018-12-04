# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure("2") do |config|
	config.vm.box = "debian/contrib-stretch64"

	config.vm.provider "virtualbox" do |v|
		if Vagrant.version?(">= 2.2.2")
			# If vagrant version is >= 2.2.2, override default nic type so it doesn't use the
			# Virtualbox default E1000 NIC types which have known vulnerabilities.
			v.default_nic_type = "virtio"
		end

		v.memory = 1024
		v.cpus = 2
	end

	config.vm.provision "shell", privileged: true, inline: <<-SCRIPT
		export DEBIAN_FRONTEND=noninteractive

		apt-get update
		apt-get install -y apt-transport-https ca-certificates curl gnupg2 software-properties-common

		export APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE=1
		curl -fsSL https://download.docker.com/linux/debian/gpg | apt-key add -
		apt-key fingerprint 0EBFCD88 | grep -q "9DC8 5822 9FC7 DD38 854A  E2D8 8D81 803C 0EBF CD88"
		if [[ $? -ne 0 ]]; then
			"ERROR: apt-key fingerprint doesn't match."
			exit 1
		fi
		add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/debian stretch stable"
		unset APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE

		apt-get update
		apt-get install -y docker-ce
		usermod -aG docker vagrant

		echo "cd /vagrant" > /etc/profile.d/change_working_directory.sh
	SCRIPT
end
