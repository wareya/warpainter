//package moe.wareya.warpainter;

import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.util.Log;
import androidx.activity.ComponentActivity;
import androidx.activity.result.ActivityResultLauncher;
import androidx.activity.result.ActivityResultCallback;
import androidx.activity.result.contract.ActivityResultContracts;
import java.io.InputStream;
import java.io.IOException;

public class FileOpenActivity extends ComponentActivity {
    public boolean finished = false;
    public boolean failed = false;
    public byte[] fileBytes;

    public final ActivityResultLauncher<String> filePicker =
        registerForActivityResult(new ActivityResultContracts.GetContent(), new ActivityResultCallback<Uri>() {
            @Override
            public void onActivityResult(Uri uri) {
                if (uri != null) {
                    fileBytes = readBytes(uri);
                    failed = fileBytes == null;
                } else {
                    failed = true;
                }
                finished = !failed;
                finish();
            }
        });

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        filePicker.launch("*/*");
    }

    public byte[] readBytes(Uri uri) {
        try (InputStream inputStream = getContentResolver().openInputStream(uri)) {
            if (inputStream == null) return null;
            return inputStream.readAllBytes();
        } catch (IOException e) {
            return null;
        }
    }
    
    public void printDebug() {
        Log.d("FileOpenActivity", "FileOpenActivity printDebug called successfully.");
    }
    static public void printDebugStatic() {
        Log.d("FileOpenActivity", "FileOpenActivity printDebugStatic called successfully.");
    }
}
