import SwiftUI

struct ProfilePicture: View {
    @State var url: String?
    @State var size: CGFloat = 48
    @State var placeholder = "person.crop.square"

    var body: some View {
        if let url = url {
            AsyncImage(
                url: URL(string: url),
                content: { image in
                    image.resizable()
                        .aspectRatio(contentMode: .fit)
                        .clipShape(RoundedRectangle(cornerSize: CGSize(width: 10, height: 10)))
                },
                placeholder: {
                    if placeholder != "" {
                        Image(systemName: placeholder)
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .frame(maxWidth: size, maxHeight: size)
                    } else {
                        Color.clear
                    }
                }
            ).frame(width: size, height: size)
        } else {
            if placeholder != "" {
                Image(systemName: placeholder)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(width: size, height: size)
            }
        }
    }
}
