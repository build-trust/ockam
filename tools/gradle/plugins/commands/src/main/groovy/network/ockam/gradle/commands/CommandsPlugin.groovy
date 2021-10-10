package network.ockam.gradle.commands

import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.tasks.Exec;
import org.gradle.api.tasks.Delete;
import java.nio.file.Paths;

class CommandsPluginExtension {
  String group = ""
  LinkedHashMap list = [:]
  ArrayList directories = []
  String rootDir = ""
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
      def workflowGenerateTaskDeps = []

      // Loop over the list of commands to define tasks for every command in the list
      commands.list.each { commandTaskName, subcommands ->
        def commandTaskDeps = []

        // Loop over the list of directories to define tasks for every directory in the list
        commands.directories.each {
          def dirPathComponents = it.path.split('/')
          def dirPath = Paths.get(*dirPathComponents)
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

          def paths = []
          def projectDirPath = project.projectDir.toString()
          def rootDirPath = Paths.get(Paths.get(projectDirPath, commands.rootDir).toFile().getCanonicalPath())

          def cargoToml = Paths.get(dirPath.toString(), 'Cargo.toml').toFile()
          if(cargoToml.exists()) {
            (cargoToml.text =~ /(?<=path\s\=\s\")(.[^\"]*)/).findAll().flatten().unique().each {
              def p = Paths.get(Paths.get(projectDirPath, dirPath.toString(), it).toFile().getCanonicalPath())
              paths << Paths.get(rootDirPath.relativize(p).toString(), '**')
            }
          }

          def mixExs = Paths.get(dirPath.toString(), 'mix.exs').toFile()
          if(mixExs.exists()) {
            (mixExs.text =~ /(?<=path:\s\")(.[^\"]*)/).findAll().flatten().unique().each {
              def p = Paths.get(Paths.get(projectDirPath, dirPath.toString(), it).toFile().getCanonicalPath())
              paths << Paths.get(rootDirPath.relativize(p).toString(), '**')
            }
          }

          ['build.gradle', 'settings.gradle'].each {
            paths << Paths.get(it)
            paths << Paths.get('implementations', project.name, it)
          }

          def p = Paths.get(Paths.get(projectDirPath, dirPath.toString()).toFile().getCanonicalPath())
          paths << Paths.get(rootDirPath.relativize(p).toString(), '**')

          def workflowFileName = "${project.name}_${dirTaskName}.yml"
          def workflowFile = Paths.get(rootDirPath.toString(), '.github', 'workflows', workflowFileName).toFile()
          paths << rootDirPath.relativize(Paths.get(workflowFile.getCanonicalPath()))

          def data = [
            "name": "${project.name}_${dirTaskName.replaceAll('\\.\\.', '')}",
            "wd": "implementations/${project.name}",
            "command": "../../gradlew ${dirTaskName}",
            "paths": paths
          ]

          def templateEngine = new groovy.text.SimpleTemplateEngine()
          def templateFile = Paths.get(rootDirPath.toString(), '.github', 'workflow_template.yml').toFile()
          def rendered = templateEngine.createTemplate(templateFile.text).make(data).toString()

          // def workflowCheckTaskName = "check_workflow_exists_for_${dirTaskName}"
          def workflowGenerateTaskName = "generate_workflow_for_${dirTaskName}"

          project.task(workflowGenerateTaskName, group: commands.group) {
            doLast { workflowFile.write(rendered) }
          }
          workflowGenerateTaskDeps.add(workflowGenerateTaskName)

          // project.task(workflowCheckTaskName, group: commands.group) {
          //   doLast {
          //     if(!workflowFile.exists()) {
          //       ant.fail("workflow file does not exist")
          //     } else if(workflowFile.text != rendered) {
          //       println "${rendered}"
          //       ant.fail("workflow is outdated")
          //     }
          //   }
          // }
          // dirTaskDeps.add(workflowCheckTaskName)

          project.task(dirTaskName, group: commands.group, dependsOn: ordered(dirTaskDeps)) {}
          commandTaskDeps.add(dirTaskName)
        }

        project.task(commandTaskName, group: commands.group, dependsOn: ordered(commandTaskDeps)) {}
      }

      project.task("generate_workflows", group: commands.group, dependsOn: ordered(workflowGenerateTaskDeps)) {}
      project.task("delete_workflows", type: Delete) {
        delete project.fileTree(dir: '../../.github/workflows', include: '**/rust_*.yml')
      }
    }
  }
}
