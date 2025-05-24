package moe.wareya.warpainter;

import android.app.Activity;
import android.app.NativeActivity;
import android.content.ContentResolver;
import android.content.Context;
import android.content.ContextWrapper;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.OpenableColumns;
import android.util.Log;
import android.view.Display;
import android.webkit.MimeTypeMap;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.IOException;

import java.lang.ref.WeakReference;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.Map;

import androidx.activity.ComponentActivity;
import androidx.activity.result.ActivityResultCallback;
import androidx.activity.result.ActivityResultLauncher;
import androidx.activity.result.contract.ActivityResultContracts;

public class FileOpenActivity extends ComponentActivity {
    public FileOpenActivity() { }
    
    public static byte[] fileBytes = null;
    public static String fileName;
    public static String fileExtension;
    
    public final ActivityResultLauncher<String[]> filePicker =
        registerForActivityResult(new ActivityResultContracts.OpenDocument(), new ActivityResultCallback<Uri>() {
            @Override
            public void onActivityResult(Uri uri) {
                
                if (uri != null) {
                    ContentResolver cR = getContentResolver();
                    MimeTypeMap mime = MimeTypeMap.getSingleton();
                    
                    Log.d("FileOpenActivity", "-- file URI: " + uri.toString());
                    String fname = getFileNameOrDummy(FileOpenActivity.this, uri);
                    Log.d("FileOpenActivity", "-- filename: " + fname);
                    String ext = fname.contains(".") ? fname.substring(fname.lastIndexOf('.') + 1) : mime.getExtensionFromMimeType(cR.getType(uri));
                    Log.d("FileOpenActivity", "-- file extension: " + ext);
                    
                    fileName = fname;
                    fileExtension = ext;
                    fileBytes = readBytes(uri);
                }
                finish();
            }
        });

    public static String getFileNameOrDummy(Context context, Uri uri) {
        String result = null;
        if ("content".equals(uri.getScheme())) {
            Cursor cursor = context.getContentResolver().query(uri, null, null, null, null);
            if (cursor != null) {
                int nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (nameIndex != -1 && cursor.moveToFirst()) {
                    result = cursor.getString(nameIndex);
                }
                cursor.close();
            }
        }
        if (result == null && "file".equals(uri.getScheme())) {
            result = new File(uri.getPath()).getName();
        }
        if (result == null || result.trim().isEmpty()) {
            result = "dummy.dat";
        }
        return result;
    }
    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        filePicker.launch(new String[]{"*/*"});
    }

    public byte[] readBytes(Uri uri) {
        try (InputStream inputStream = getContentResolver().openInputStream(uri)) {
            if (inputStream == null) return null;
            return inputStream.readAllBytes();
        } catch (IOException e) {
            return null;
        }
    }
    static public void clearBytes() {
        fileBytes = null;
    }
    
    public void printDebug() {
        Log.d("FileOpenActivity", "FileOpenActivity printDebug called successfully.");
    }
    static public void printDebugStatic() {
        Log.d("FileOpenActivity", "FileOpenActivity printDebugStatic called successfully.");
    }
}
