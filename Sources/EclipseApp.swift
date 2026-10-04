
import SwiftUI

@main
struct EclipseApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

struct ContentView: View {
    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            VStack(spacing: 20) {
                Text("SUPER MARIO")
                    .font(.title.bold())
                    .foregroundColor(.white)

                Text("ECLIPSE")
                    .font(.system(size: 48, weight: .black))
                    .foregroundColor(.yellow)

                Text("Native iOS Development Build")
                    .foregroundColor(.gray)

                Button("START GAME") {
                    print("Native engine not integrated yet")
                }
                .buttonStyle(.borderedProminent)
            }
        }
    }
}
