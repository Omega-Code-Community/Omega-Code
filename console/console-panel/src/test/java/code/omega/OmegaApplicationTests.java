package code.omega;

import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.jdbc.core.JdbcTemplate;

import javax.sql.DataSource;
import java.sql.Connection;
import java.sql.SQLException;

@SpringBootTest
class OmegaApplicationTests {

	@Autowired
	private DataSource dataSource;

	@Autowired
	private JdbcTemplate jdbcTemplate;

	@Test
	public void testDataSourceConnection() throws SQLException {
		// 1. 测试获取连接
		try (Connection connection = dataSource.getConnection()) {
			System.out.println("数据库连接成功");
			System.out.println("数据源类型: " + dataSource.getClass().getName());
			System.out.println("连接URL: " + connection.getMetaData().getURL());
		}
	}

	@Test
	public void testJdbcTemplate() {
		jdbcTemplate.execute("SELECT 1");
		System.out.println("SQL执行成功，数据库正常工作！");
	}
}
