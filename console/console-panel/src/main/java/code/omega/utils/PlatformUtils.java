package code.omega.utils;

import lombok.extern.slf4j.Slf4j;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;

/**
 * @FileName PlatformUtils
 * @Description
 * @Author Gabriel
 * @date 2026-05-25
 **/
@Slf4j
public class PlatformUtils {

    private static final String APP_DIR_NAME = ".omega";
    private static final String DATABASE_NAME = "omega_code.db";
    private static final String STATIC_DIR_NAME = "static";

    private PlatformUtils() {
    }

    /**
     * 获取用户主目录
     * Windows: C:\Users\XXX
     * Mac/Linux: /home/XXX
     */
    public static String getUserDir() {
        return System.getProperty("user.home");
    }

    /**
     * 获取应用根目录 ~/.omega
     */
    public static String getAppDir() {
        return getUserDir() + "/" + APP_DIR_NAME;
    }

    /**
     * 获取数据库完整路径 ~/.omega/omega_code.db
     */
    public static String getDatabasePath() {
        return getAppDir() + "/" + DATABASE_NAME;
    }

    /**
     * 获取系统临时目录
     */
    public static String getOsTempDir() {
        return System.getProperty("java.io.tmpdir");
    }

    /**
     * 静态资源目录 ~/.omega/static
     */
    public static String getStaticDir() {
        String staticDir = getAppDir() + "/" + STATIC_DIR_NAME;
        try {
            if (!Files.exists(Paths.get(staticDir))) {
                Files.createDirectories(Paths.get(staticDir));
                log.info("静态资源目录创建成功：{}", staticDir);
            }
        } catch (IOException e) {
            log.error("静态资源目录创建失败：{}", e.getMessage(), e);
            throw new RuntimeException("静态资源目录初始化失败", e);
        }
        return staticDir;
    }
}