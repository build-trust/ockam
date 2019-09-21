package network.ockam.gradle.configuration

import groovy.json.JsonSlurper

import org.gradle.api.Plugin
import org.gradle.api.Project

class ConfigurationPlugin implements Plugin<Project> {

  void apply(Project project) {
    def home = project.hasProperty('ockamHome') ?: (new File(System.properties['user.home'], '.ockam')).getPath()
    def configFile = new File(home, 'config.json')

    def jsonSlurper = new JsonSlurper()
    if(configFile.exists() && configFile.canRead()) {
      project.ext.configuration = jsonSlurper.parse(configFile)
    }
  }

}
