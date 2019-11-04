package network.ockam.gradle.host

import org.gradle.api.Plugin
import org.gradle.api.Project

import groovy.json.JsonSlurper

class HostPlugin implements Plugin<Project> {

  void apply(Project project) {
    def home = project.hasProperty('ockamHome') ?: (new File(System.properties['user.home'], '.ockam')).getPath()
    def configFile = new File(home, 'build_host.json')

    def jsonSlurper = new JsonSlurper()
    def config = [
      testInventory: []
    ]
    if(configFile.exists() && configFile.canRead()) {
      try {
        config = jsonSlurper.parse(configFile)
      } catch (Exception e) {}
    }

    project.ext.host = config
  }

}
