package network.ockam.gradle.commands

import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.tasks.Exec;

class CommandsPluginExtension {
  String group = ""
  LinkedHashMap list = [:]
  ArrayList directories = []
}

class CommandsPlugin implements Plugin<Project> {

  void apply(Project project) {

    def ordered = { dependencies ->
      for (int i = 0; i < dependencies.size() - 1; i++) {
        def current = project.tasks.named(dependencies[i]).get()
        def next = project.tasks.named(dependencies[i + 1]).get()
        next.mustRunAfter(current)
      }
      dependencies
    }

    // Add the 'commands' extension object
    def commands = project.extensions.create('commands', CommandsPluginExtension)

    project.afterEvaluate {

      // Loop over the list of commands to define tasks for every command in the list
      commands.list.each { commandTaskName, subcommands ->
        def commandTaskDeps = []

        commands.directories.each {
          def dirPathComponents = it.split('/')
          def dirPath = java.nio.file.Paths.get(*dirPathComponents)
          def dirName = dirPathComponents.join('_')
          def dirTaskName = [commandTaskName, dirName].join('_')
          def dirTaskDeps = []

          // there may be one or more subcommands
          // turn the subcommands into an array if it is a single subcommand
          subcommands = ([] + subcommands)
          subcommands.eachWithIndex { subcommand, index ->
            def subCommandTaskName = "__${dirTaskName}_${index}"

            project.task(subCommandTaskName, type: Exec) {
              commandLine subcommand.split()
              workingDir dirPath
            }

            dirTaskDeps.add(subCommandTaskName)
          }

          project.task(dirTaskName, group: commands.group, dependsOn: ordered(dirTaskDeps)) {}
          commandTaskDeps.add(dirTaskName)
        }

        project.task(commandTaskName, group: commands.group, dependsOn: ordered(commandTaskDeps)) {}
      }
    }
  }
}
