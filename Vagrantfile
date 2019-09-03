Vagrant.configure("2") do |config|

  ockam_private_boxes_shared_access_token = ENV['OCKAM_PRIVATE_BOXES_SHARED_ACCESS_TOKEN']

  if ockam_private_boxes_shared_access_token
    config.vm.define "builder-macos", primary: true do |config|
      config.vm.box = "ockam-network/builder-macos"

      boxes_url = "https://ockam.blob.core.windows.net/boxes-private"
      sha256 = "046a286dff1fd80814dc2f30c2478d636d1796f590c29562a2ffa1763590a507"
      config.vm.box_url = "#{boxes_url}/builder/macos/#{sha256}.box?#{ockam_private_boxes_shared_access_token}"
      config.vm.box_download_checksum = sha256
      config.vm.box_download_checksum_type = "sha256"
      config.vm.box_check_update = false

      config.vm.provider :virtualbox do |vbox|
        vbox.name = "builder-macos"
        vbox.linked_clone = true
        vbox.customize ['modifyvm', :id, '--usb', 'on']
      end

      config.vm.synced_folder ".", "/vagrant", type: :rsync, rsync__exclude: ['.git/', 'tools/builder/']
    end
  end

end
