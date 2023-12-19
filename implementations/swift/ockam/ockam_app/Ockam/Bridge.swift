// This files is a bridge between the C library and the swift representation.
// It takes the C data structures from Bridge.h and re-define them using native
// swift elements such as string and arrays and convert them from C.
//
// The optionality of fields is lost in C structures and must be manually
// restored on a field-by-field basis.
//
// It also expose wrappers prefixed with swift_, like swift_initialize_application().

import Foundation
import UserNotifications

class Invitee: Identifiable, Hashable, Equatable, ObservableObject {
    @Published var name: Optional<String>
    let email: String

    var id: String { email }

    init(name: String?, email: String) {
        self.name = name
        self.email = email
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(name)
        hasher.combine(email)
    }

    static func == (lhs: Invitee, rhs: Invitee) -> Bool {
        return lhs.name == rhs.name && lhs.email == rhs.email
    }
}

class Invitation: Identifiable, Hashable, Equatable, ObservableObject {
    let id: String
    @Published var serviceName: String
    @Published var serviceScheme: String?
    @Published var accepting: Bool
    @Published var accepted: Bool
    @Published var ignoring: Bool

    init(
        id: String, serviceName: String, serviceScheme: String?, accepting: Bool, accepted: Bool,
        ignoring: Bool
    ) {
        self.id = id
        self.serviceName = serviceName
        self.serviceScheme = serviceScheme
        self.accepting = accepting
        self.accepted = accepted
        self.ignoring = ignoring
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
        hasher.combine(serviceName)
        hasher.combine(serviceScheme)
        hasher.combine(accepting)
        hasher.combine(accepted)
        hasher.combine(ignoring)
    }

    static func == (lhs: Invitation, rhs: Invitation) -> Bool {
        return lhs.id == rhs.id && lhs.serviceName == rhs.serviceName
        && lhs.serviceScheme == rhs.serviceScheme && lhs.accepting == rhs.accepting
        && lhs.accepted == rhs.accepted && lhs.ignoring == rhs.ignoring
    }
}

class LocalService: Identifiable, Hashable, Equatable, ObservableObject {
    let name: String
    @Published var address: String
    @Published var port: UInt16
    @Published var scheme: String?
    @Published var sharedWith: [Invitee]
    @Published var available: Bool

    var id: String { name }

    init(
        name: String, address: String, port: UInt16, scheme: String?, sharedWith: [Invitee],
        available: Bool
    ) {
        self.name = name
        self.address = address
        self.port = port
        self.scheme = scheme
        self.sharedWith = sharedWith
        self.available = available
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(name)
        hasher.combine(address)
        hasher.combine(port)
        hasher.combine(scheme)
        hasher.combine(sharedWith)
        hasher.combine(available)
    }

    static func == (lhs: LocalService, rhs: LocalService) -> Bool {
        return lhs.name == rhs.name && lhs.address == rhs.address && lhs.port == rhs.port
        && lhs.scheme == rhs.scheme && lhs.sharedWith == rhs.sharedWith
        && lhs.available == rhs.available
    }
}

class Service: Identifiable, Hashable, Equatable, ObservableObject {
    @Published var sourceName: String
    @Published var address: String?
    @Published var port: UInt16?
    @Published var scheme: String?
    @Published var available: Bool
    @Published var enabled: Bool
    let id: String

    init(
        sourceName: String, address: String? = nil, port: UInt16? = nil, scheme: String? = nil,
        available: Bool = false, enabled: Bool = false, id: String
    ) {
        self.sourceName = sourceName
        self.address = address
        self.port = port
        self.scheme = scheme
        self.available = available
        self.enabled = enabled
        self.id = id
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(sourceName)
        hasher.combine(address)
        hasher.combine(port)
        hasher.combine(scheme)
        hasher.combine(available)
        hasher.combine(enabled)
        hasher.combine(id)
    }

    static func == (lhs: Service, rhs: Service) -> Bool {
        return lhs.sourceName == rhs.sourceName && lhs.address == rhs.address
        && lhs.port == rhs.port && lhs.scheme == rhs.scheme && lhs.available == rhs.available
        && lhs.enabled == rhs.enabled && lhs.id == rhs.id
    }
}

class ServiceGroup: Identifiable, Hashable, Equatable, ObservableObject {
    @Published var name: String?
    let email: String
    @Published var imageUrl: String?
    @Published var invitations: [Invitation]
    @Published var incomingServices: [Service]

    var id: String { email }

    init(
        name: String? = nil, email: String, imageUrl: String? = nil, invitations: [Invitation] = [],
        incomingServices: [Service] = []
    ) {
        self.name = name
        self.email = email
        self.imageUrl = imageUrl
        self.invitations = invitations
        self.incomingServices = incomingServices
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(name)
        hasher.combine(email)
        hasher.combine(imageUrl)
        hasher.combine(invitations)
        hasher.combine(incomingServices)
    }

    static func == (lhs: ServiceGroup, rhs: ServiceGroup) -> Bool {
        return lhs.name == rhs.name && lhs.email == rhs.email && lhs.imageUrl == rhs.imageUrl
        && lhs.invitations == rhs.invitations && lhs.incomingServices == rhs.incomingServices
    }
}

enum OrchestratorStatus: Int {
    case Disconnected = 0
    case Connecting
    case Connected
    case WaitingForToken
    case WaitingForEmailValidation
    case RetrievingSpace
    case RetrievingProject
}

class ApplicationState: ObservableObject, CustomDebugStringConvertible {
    @Published var enrolled: Bool
    @Published var loaded: Bool
    @Published var orchestrator_status: OrchestratorStatus
    @Published var enrollmentName: String?
    @Published var enrollmentEmail: String?
    @Published var enrollmentImage: String?
    @Published var enrollmentGithubUser: String?
    @Published var localServices: [LocalService]
    @Published var groups: [ServiceGroup]
    @Published var sent_invitations: [Invitee]

    init(
        enrolled: Bool,
        loaded: Bool,
        orchestrator_status: OrchestratorStatus,
        enrollmentName: String?,
        enrollmentEmail: String?,
        enrollmentImage: String?,
        enrollmentGithubUser: String?,
        localServices: [LocalService],
        groups: [ServiceGroup],
        sent_invitations: [Invitee]
    ) {
        self.enrolled = enrolled
        self.loaded = loaded
        self.orchestrator_status = orchestrator_status
        self.enrollmentName = enrollmentName
        self.enrollmentEmail = enrollmentEmail
        self.enrollmentImage = enrollmentImage
        self.enrollmentGithubUser = enrollmentGithubUser
        self.localServices = localServices
        self.groups = groups
        self.sent_invitations = sent_invitations
    }

    func getLocalService(_ localServiceId: String) -> LocalService? {
        for service in self.localServices {
            if service.id == localServiceId {
                return service
            }
        }
        return nil
    }

    func lookupInvitationById(_ invitationId: String) -> (ServiceGroup, Invitation)? {
        for group in self.groups {
            for invitation in group.invitations {
                if invitation.id == invitationId {
                    return (group, invitation)
                }
            }
        }
        return nil
    }

    func lookupIncomingServiceById(_ serviceId: String) -> (ServiceGroup, Service)? {
        for group in self.groups {
            for service in group.incomingServices {
                if service.id == serviceId {
                    return (group, service)
                }
            }
        }
        return nil
    }
}

enum NotificationKind: Int {
    case information = 0
    case warning = 1
    case error = 2
}

struct Notification {
    var kind: NotificationKind
    var title: String
    var message: String
}

func swift_demo_application_state() -> ApplicationState {
    return convertApplicationState(cState: mock_application_state())
}

func swift_application_snapshot() -> ApplicationState {
    return convertApplicationState(cState: application_state_snapshot())
}

func swift_initialize_application() -> Bool {
    let applicationStateClosure: @convention(c) (C_ApplicationState) -> Void = { state in
        StateContainer.shared.update(state: convertApplicationState(cState: state))
    }

    let notificationClosure: @convention(c) (C_Notification) -> Void = { cNotification in
        let notification = convertNotification(cNotification: cNotification)

        let content = UNMutableNotificationContent()
        content.title = notification.title
        content.body = notification.message

        let request = UNNotificationRequest(
            identifier: UUID().uuidString, content: content, trigger: nil)

        UNUserNotificationCenter.current().add(request)
    }

    let result = initialize_application(applicationStateClosure, notificationClosure)

    UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) {
        granted, error in
        if granted == true {
            print("Notification permission granted")
        } else {
            print("Notifications not allowed")
        }
    }

    return result
}

func optional_string(str: UnsafePointer<Int8>?) -> String? {
    guard let str = str else { return nil }
    return String(cString: str)
}

func convertNotification(cNotification: C_Notification) -> Notification {
    let kind = NotificationKind(rawValue: Int(cNotification.kind.rawValue))!
    let title = String(cString: cNotification.title)
    let message = String(cString: cNotification.message)

    return Notification(kind: kind, title: title, message: message)
}

func convertApplicationState(cState: C_ApplicationState) -> ApplicationState {
    let enrollmentName = optional_string(str: cState.enrollment_name)
    let enrollmentEmail = optional_string(str: cState.enrollment_email)
    let enrollmentImage = optional_string(str: cState.enrollment_image)
    let enrollmentGithubUser = optional_string(str: cState.enrollment_github_user)

    var localServices: [LocalService] = []
    var i = 0
    while let cLocalService = cState.local_services[i] {
        localServices.append(convertLocalService(cLocalService: cLocalService))
        i += 1
    }

    var groups: [ServiceGroup] = []
    i = 0
    while let cGroup = cState.groups[i] {
        groups.append(convertServiceGroup(cServiceGroup: cGroup))
        i += 1
    }

    var sent_invitations: [Invitee] = []
    i = 0
    while let cInvitee = cState.sent_invitations[i] {
        sent_invitations.append(convertInvitee(cInvitee: cInvitee))
        i += 1
    }

    return ApplicationState(
        enrolled: cState.enrolled != 0,
        loaded: cState.loaded != 0,
        orchestrator_status: OrchestratorStatus(
            rawValue: Int(cState.orchestrator_status.rawValue))!,
        enrollmentName: enrollmentName,
        enrollmentEmail: enrollmentEmail,
        enrollmentImage: enrollmentImage,
        enrollmentGithubUser: enrollmentGithubUser,
        localServices: localServices,
        groups: groups,
        sent_invitations: sent_invitations
    )
}

func convertLocalService(cLocalService: UnsafePointer<C_LocalService>) -> LocalService {
    let cService = cLocalService.pointee

    let name = String(cString: cService.name)
    let address = String(cString: cService.address)
    let scheme = optional_string(str: cService.scheme)

    var sharedWith: [Invitee] = []
    var i = 0
    while let cInvitee = cService.shared_with[i] {
        sharedWith.append(convertInvitee(cInvitee: cInvitee))
        i += 1
    }

    return LocalService(
        name: name,
        address: address,
        port: cService.port,
        scheme: scheme,
        sharedWith: sharedWith,
        available: cService.available != 0
    )
}

func convertInvitee(cInvitee: UnsafePointer<C_Invitee>) -> Invitee {
    let cRecord = cInvitee.pointee

    let name = optional_string(str: cRecord.name)
    let email = String(cString: cRecord.email)

    return Invitee(name: name, email: email)
}

func convertServiceGroup(cServiceGroup: UnsafePointer<C_ServiceGroup>) -> ServiceGroup {
    let cGroup = cServiceGroup.pointee

    let name = optional_string(str: cGroup.name)
    let email = String(cString: cGroup.email)
    let imageUrl = optional_string(str: cGroup.image_url)

    var invitations: [Invitation] = []
    var i = 0
    while let cInvite = cGroup.invitations[i] {
        invitations.append(convertInvitation(cInvite: cInvite))
        i += 1
    }

    var incomingServices: [Service] = []
    i = 0
    while let cService = cGroup.incoming_services[i] {
        incomingServices.append(convertService(cService: cService))
        i += 1
    }

    return ServiceGroup(
        name: name,
        email: email,
        imageUrl: imageUrl,
        invitations: invitations,
        incomingServices: incomingServices
    )
}

func convertInvitation(cInvite: UnsafePointer<C_Invitation>) -> Invitation {
    let cRecord = cInvite.pointee

    let id = String(cString: cRecord.id)
    let serviceName = String(cString: cRecord.service_name)
    let serviceScheme = optional_string(str: cRecord.service_scheme)
    let accepting = cRecord.accepting != 0
    let accepted = cRecord.accepted != 0
    let ignoring = cRecord.ignoring != 0

    return Invitation(
        id: id, serviceName: serviceName, serviceScheme: serviceScheme, accepting: accepting,
        accepted: accepted, ignoring: ignoring)
}

func convertService(cService: UnsafePointer<C_Service>) -> Service {
    let cRecord = cService.pointee

    let sourceName = String(cString: cRecord.source_name)
    let address = optional_string(str: cRecord.address)
    let scheme = optional_string(str: cRecord.scheme)
    let id = String(cString: cRecord.id)
    let port = cRecord.port == 0 ? nil : Optional(cRecord.port)

    return Service(
        sourceName: sourceName,
        address: address,
        port: port,
        scheme: scheme,
        available: cRecord.available != 0,
        enabled: cRecord.enabled != 0,
        id: id
    )

}

extension Invitee: CustomDebugStringConvertible {
    var debugDescription: String {
        return "{ \"name\": \"\(name ?? "nil")\", \"email\": \"\(email)\" }"
    }
}

extension Invitation: CustomDebugStringConvertible {
    var debugDescription: String {
        return "{ \"id\": \"\(id)\", \"serviceName\": \"\(serviceName)\", \"serviceScheme\": \"\(serviceScheme ?? "nil")\", \"accepting\": \(accepting) }"
    }
}

extension LocalService: CustomDebugStringConvertible {
    var debugDescription: String {
        let sharedWithJsonStrings = sharedWith.map { $0.debugDescription }.joined(separator: ", ")
        return "{ \"name\": \"\(name)\", \"address\": \"\(address)\", \"port\": \(port), \"scheme\": \"\(scheme ?? "none")\", \"sharedWith\": [ \(sharedWithJsonStrings) ], \"available\": \(available) }"
    }
}

extension Service: CustomDebugStringConvertible {
    var debugDescription: String {
        return "{ \"sourceName\": \"\(sourceName)\", \"address\": \"\(address ?? "nil")\", \"port\": \(String(describing: port)), \"scheme\": \"\(scheme ?? "nil")\", \"available\": \(available), \"enabled\": \(enabled), \"id\": \"\(id)\" }"
    }
}

extension ServiceGroup: CustomDebugStringConvertible {
    var debugDescription: String {
        let invitationsStrings = invitations.map { $0.debugDescription }.joined(separator: ", ")
        let incomingServicesStrings = incomingServices.map { $0.debugDescription }.joined(
            separator: ", ")
        return "{ \"name\": \"\(name ?? "nil")\", \"email\": \"\(email)\", \"imageUrl\": \"\(imageUrl ?? "nil")\", \"invitations\": [ \(invitationsStrings) ], \"incomingServices\" : [ \(incomingServicesStrings) ] }"
    }
}

extension ApplicationState {
    var debugDescription: String {
        let localServicesStrings = localServices.map { $0.debugDescription }.joined(separator: ", ")
        let groupsStrings = groups.map { $0.debugDescription }.joined(separator: ", ")
        return "{ \"enrolled\": \(enrolled), \"loaded\": \(loaded), \"orchestrator_status\": \(orchestrator_status.rawValue), \"enrollmentName\": \"\(enrollmentName ?? "nil")\", \"enrollmentEmail\": \"\(enrollmentEmail ?? "nil")\", \"enrollmentImage\": \"\(enrollmentImage ?? "nil")\", \"enrollmentGithubUser\": \"\(enrollmentGithubUser ?? "nil")\", \"localServices\": [ \(localServicesStrings) ], \"groups\": [ \(groupsStrings) ], \"sent_invitations\": [ \(sent_invitations) ] }"
    }
}

class RuntimeInformation {
    let version: String
    let commit: String
    let home: String?
    let controllerAddr: String?
    let controllerIdentity: String?

    init(version: String, commit: String, home: String?, controllerAddr: String?, controllerIdentity: String?) {
        self.version = version
        self.commit = commit
        self.home = home
        self.controllerAddr = controllerAddr
        self.controllerIdentity = controllerIdentity
    }
}

func swift_runtime_information() -> RuntimeInformation {
    let cRuntimeInformation = runtime_information()
    let version = String(cString: cRuntimeInformation.version)
    let commit = String(cString: cRuntimeInformation.commit)
    let home = optional_string(str: cRuntimeInformation.home)
    let controllerAddr = optional_string(str: cRuntimeInformation.controller_addr)
    let controllerIdentity = optional_string(str: cRuntimeInformation.controller_identity)

    let info = RuntimeInformation(version: version, commit: commit, home: home, controllerAddr: controllerAddr, controllerIdentity: controllerIdentity)
    free_runtime_information(cRuntimeInformation)

    return info
}
