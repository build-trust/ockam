# macOS Builder

## Prerequisites

Install the latest version of [Virtualbox](https://www.virtualbox.org).
```
brew cask install virtualbox
```

Install [Virtualbox Extensions](https://www.virtualbox.org/wiki/Downloads).
```
brew cask install virtualbox-extension-pack
```

Install the latest version of [Vagrant](https://www.vagrantup.com).
```
brew cask install vagrant
```

Install the latest version of [Macinbox](https://github.com/bacongravy/macinbox).
```
sudo gem install macinbox
```

## Build

```
./clean && ./build "Install macOS Mojave.app"
```
