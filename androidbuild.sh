#!sh

set -ex

export ANDROID_HOME="C:/Users/wareya/AppData/Local/Android/Sdk/"

#export ANDROID_NDK_HOME="C:/Users/wareya/AppData/Local/Android/Sdk/ndk/29.0.13113456/"
#export SDK_VER="35.0.1"

#rustup target add aarch64-linux-android
#cargo install cargo-ndk

cargo ndk -t arm64-v8a -o android/lib/ build

javac -classpath "$ANDROID_HOME/platforms/android-35/android.jar;.trash/*" src/FileOpenActivity.java -d .trash/
java -cp ".trash/*;$ANDROID_HOME/build-tools/35.0.1/lib/d8.jar" com.android.tools.r8.D8 --output src/data/ .trash/FileOpenActivity*.class .trash/*.jar --no-desugaring --min-api 30
touch -t 198001010000 src/data/classes.dex # deterministic
jar cvf src/data2/fileopenactivity.jar -C src/data/ classes.dex

# don't remember where i found this
patch_zip_timestamps() {
    local file="$1"
    local sig local start step offset pos
    patch() {
        sig="$1"
        start="$2"
        step="$3"
        offset=0
        while true; do
            pos=$(grep -ab -o "$sig" "$file" | awk -F: -v min=$offset '$1 >= min {print $1; exit}')
            [ -z "$pos" ] && break
            dd if=/dev/zero bs=1 count=4 seek=$((pos + start)) conv=notrunc of="$file" 2>/dev/null
            offset=$((pos + step))
        done
    }
    patch $'\x50\x4b\x03\x04' 10 30
    patch $'\x50\x4b\x01\x02' 12 46
}

echo "deterministicifying jar..."
patch_zip_timestamps src/data2/fileopenactivity.jar
echo "deterministicied"

cp src/data2/* android/assets

"$ANDROID_HOME/build-tools/35.0.1/aapt2" link -I "$ANDROID_HOME/platforms/android-35/android.jar" --manifest android/AndroidManifest.xml -o target/warpainter-unsigned.apk

cd android
zip -r ../target/warpainter-unsigned.apk . -x "AndroidManifest.xml"
cd ..

rm target/warpainter-aligned.apk
"$ANDROID_HOME/build-tools/35.0.1/zipalign" -v 4 target/warpainter-unsigned.apk target/warpainter-aligned.apk

java -jar "$ANDROID_HOME/build-tools/35.0.1/lib/apksigner.jar" sign --ks ~/.android/debug.keystore \
    --ks-key-alias androiddebugkey --ks-pass pass:android --key-pass pass:android \
    --out target/warpainter-signed.apk target/warpainter-aligned.apk

adb install target/warpainter-signed.apk

adb logcat -c && adb shell am start -n moe.wareya.warpainter/android.app.NativeActivity && adb logcat | grep -iP "moe.wareya.warpainter| rust|[\w]System|FileOpen"

