package network.ockam.gradle.builders

import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.NamedDomainObjectContainer
import org.gradle.api.Task
import org.gradle.api.tasks.Exec
import org.gradle.api.tasks.Delete

import groovy.io.FileType
import groovy.json.JsonSlurper

class BuilderExecSpec {
  String scriptText
  String workingDirPath

  void script(String s) {
    scriptText = s
  }

  void workingDir(String p) {
    workingDirPath = p
  }
}

class BuildersPlugin implements Plugin<Project> {

  void apply(Project project) {
    def config = [
      debianBuilder: [
        enabled: true,
        memory: 4096,
        cpus: 2
      ],
      macosBuilder: [
        enabled: false,
        memory: 4096,
        cpus: 2
      ]
    ]

    def currentDir = project.file('.')
    def vagrantfileDir = findVagrantfileDir(currentDir)
    def pathRelativeToVagrentfileDir = vagrantfileDir.toPath().relativize(currentDir.toPath()).toFile()

    config = merge(config, project.host)
    config = merge(config, [
      vagrantfileDir: vagrantfileDir,
      currentDir: currentDir,
      pathRelativeToVagrentfileDir: pathRelativeToVagrentfileDir
    ])
    project.host = config

    project.extensions.add('builderExec', { builderName, closure ->
      isRunning(project, builderName) ? syncVM(project, builderName) : startVM(project, builderName)
      execute(project, builderName, closure)
      syncHost(project, builderName)
    })
  }

  static boolean usingDockerProvider() {
    def env = System.getenv("VAGRANT_DEFAULT_PROVIDER")
    return env == "docker";
  }

  static boolean isRunning(Project project, String builderName) {
    def output = new ByteArrayOutputStream()
    project.exec {
      commandLine 'vagrant', 'status', "builder-${builderName}"
      standardOutput = output
    }
    return output.toString().tokenize().contains('running')
  }

  // starting from the current directory, look for a parent directory that has a Vagrantfile
  static File findVagrantfileDir(File currentDir) {
    def path = currentDir
    def hasVagrantfile = false
    while (true) {
      path.eachFileMatch (FileType.FILES, /Vagrantfile/) { file ->
        hasVagrantfile = true
      }
      if (hasVagrantfile) break
      path = new File('..', path)
    }
    return (new File(path.getCanonicalPath()))
  }

  static Map merge(Map onto, Map... overrides) {
    if (!overrides)
      return onto
    else if (overrides.length == 1) {
      overrides[0]?.each { k, v ->
        if (v instanceof Map && onto[k] instanceof Map)
          merge((Map) onto[k], (Map) v)
        else
        onto[k] = v
      }
      return onto
    }
    return overrides.inject(onto, { acc, override -> merge(acc, override ?: [:]) })
  }

  static startVM(Project project, String builderName) {
    def sshConfigPath = (new File('.builder', "${builderName}.ssh-config")).toString()
    sshConfigPath = (new File(project.host.vagrantfileDir.toString(), sshConfigPath)).toString()

    def env = [
      ("OCKAM_${builderName.toUpperCase()}_BUILDER_MEMORY".toString()): (project.host["${builderName}Builder"]).memory,
      ("OCKAM_${builderName.toUpperCase()}_BUILDER_CPUS".toString()): (project.host["${builderName}Builder"]).cpus
    ]
    if(project.host.privateBoxesSharedAccessToken != null) {
      env['OCKAM_PRIVATE_BOXES_SHARED_ACCESS_TOKEN'] = project.host.privateBoxesSharedAccessToken
    }

    def builderDir = new File(project.host.vagrantfileDir.toString(), '.builder')
    builderDir.mkdirs()

    project.exec {
      environment env
      workingDir project.host.vagrantfileDir
      commandLine 'vagrant', 'up', "builder-${builderName}"
    }
    project.exec {
      standardOutput = new FileOutputStream(sshConfigPath)
      commandLine 'vagrant', 'ssh-config', "builder-${builderName}"
    }
  }

  static syncVM(Project project, String builderName) {
    if (usingDockerProvider()) {
      return;
    }
    project.exec {
      workingDir project.host.vagrantfileDir
      commandLine adaptCommandForOs('vagrant', 'rsync', "builder-${builderName}")
    }
  }

  static List adaptCommandForOs(String... command) {
    def newCommand = []
    if (System.getProperty('os.name').toLowerCase(Locale.ROOT).contains('windows')) {
      newCommand = ['cmd', '/c']
    }
    newCommand.addAll(command)
    return newCommand
  }

  static execute(Project project, String builderName, Closure configureClosure) {
    def builderExecSpec = new BuilderExecSpec()
    builderExecSpec.with configureClosure

    def pathRelativeToVagrentfileDir = project.host.pathRelativeToVagrentfileDir ?: ""
    def workingDirPath = builderExecSpec.workingDirPath ?: ""
    def scriptText = builderExecSpec.scriptText

    def script = "cd /vagrant/${pathRelativeToVagrentfileDir}/${workingDirPath} && ${scriptText}"

    project.exec {
      workingDir project.host.vagrantfileDir
      commandLine adaptCommandForOs('vagrant', 'ssh', "builder-${builderName}", '-c', script)
    }
  }

  static syncHost(Project project, String builderName) {
    if (usingDockerProvider()) {
      return;
    }
    def vDir = project.host.vagrantfileDir.toString()
    def result = project.exec {
      workingDir project.host.vagrantfileDir
      commandLine adaptCommandForOs('rsync',
        '--exclude', '.git',
        '--exclude', '.vagrant',
        '--exclude', 'tools/builder/',
        '--exclude', '.builder',
        '--delete', '-v', //'--stats', // '--progress',
        '-r', '-a',
        '-e', "ssh -o ServerAliveInterval=5 -o ServerAliveCountMax=1000 -F ${vDir}/.builder/${builderName}.ssh-config",
        "builder-${builderName}:/vagrant/", '.')
      ignoreExitValue true
    }
    if (result.getExitValue() != 0) {
      syncHost(project, builderName)
    }
  }
}
