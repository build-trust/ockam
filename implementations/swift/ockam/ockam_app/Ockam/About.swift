import SwiftUI

struct About: View {
    @Environment(\.presentationMode) var presentationMode: Binding<PresentationMode>
    @State var runtimeInformation: RuntimeInformation

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 0) {
                Image("Logo")
                    .frame(width: 128, height: 128)
                    .padding(.vertical, VerticalSpacingUnit)
                    .padding(.vertical, HorizontalSpacingUnit)

                Spacer()
                Grid(alignment: .leading) {
                    GridRow {
                        Text("Version:")
                        Text(verbatim: runtimeInformation.version)
                            .textSelection(.enabled)
                    }
                    GridRow {
                        Text("Commit:")
                        Text(verbatim: runtimeInformation.commit)
                            .textSelection(.enabled)
                    }
                    if let home = runtimeInformation.home {
                        GridRow {
                            Text("Home:")
                            Text(verbatim: home)
                                .textSelection(.enabled)
                        }
                    }
                    if let controllerAddr = runtimeInformation.controllerAddr {
                        GridRow {
                            Text("Controller:")
                            Text(verbatim: controllerAddr)
                                .textSelection(.enabled)
                        }
                    }
                    if let controllerIdentity = runtimeInformation.controllerIdentity {
                        GridRow {
                            Text("Controller Identity:")
                            Text(verbatim: controllerIdentity)
                                .textSelection(.enabled)
                        }
                    }
                    Spacer()
                }
                .padding(.vertical, VerticalSpacingUnit)
                .padding(.horizontal, HorizontalSpacingUnit)
            }

            HStack {
                Spacer()
                Button(
                    action: {
                        self.closeWindow()
                    },
                    label: {
                        Text("Dismiss")
                    }
                )
                .keyboardShortcut(.defaultAction)
                .padding(.vertical, VerticalSpacingUnit)
                .padding(.horizontal, HorizontalSpacingUnit)
            }
            .background(OckamDarkerBackground)
            .padding(0)
        }
        .frame(width: 650, height: 330)
        .padding(0)
    }

    func closeWindow() {
        presentationMode.wrappedValue.dismiss()
    }
}

struct About_Previews: PreviewProvider {
    static var previews: some View {
        VStack {
            About(runtimeInformation: RuntimeInformation.init(
                version: "11.22.33",
                commit: "7d866ec4dbcb238094480a142a7b471f6971a368",
                home: "/tmp/ockam/my-alternative-home",
                controllerAddr: "/dnsaddr/..../tcp/1234/service/api",
                controllerIdentity: "I42b962f5b4e4b6693c3796a20572ea64d4f7a9a0ed681518b1c3723ebf685b90"
            ))
            Divider()
            About(runtimeInformation: swift_runtime_information())
        }
        .frame(width: 650, height: 700)
    }
}
