import {ExcelProps} from "../../model/ExcelProps";
import {AspectRatio, Badge, Button, Card, Group, Image, Text, useMantineTheme} from "@mantine/core";
import {useRouter} from "next/router";
import React from "react";

interface IParams {
    excelProps: ExcelProps;
    showDate?: boolean;
}

export default function ExcelCardItem(params: IParams): JSX.Element {
    const {excelProps, showDate} = params;
    const theme = useMantineTheme();
    const router = useRouter()

    const secondaryColor = theme.colorScheme === 'dark'
        ? theme.colors.dark[1]
        : theme.colors.gray[7];

    return (
        <div style={{width: 340, margin: 'auto'}}>
            <Card shadow="xl" p="lg" radius="md">
                <Card.Section>
                    <AspectRatio ratio={1280 / 720}>
                        <Image
                            src={"https://i.ytimg.com/vi/" + excelProps.youtubeUrl.replace("https://www.youtube.com/watch?v=", "") + "/hqdefault.jpg"}
                            width={"100%"}
                            height={"100%"}
                            alt="" imageProps={{"loading": "lazy"}}/>
                    </AspectRatio>
                </Card.Section>

                <Group position="apart" style={{marginBottom: 5, marginTop: theme.spacing.sm}}>
                    <Text weight={500}>{excelProps.name}</Text>
                    {showDate &&
                        <Badge color="pink" variant="light">
                            {(new Date(excelProps.date)).toLocaleDateString('ro-RO')}
                        </Badge>
                    }
                </Group>

                <Text size="sm" style={{color: secondaryColor, lineHeight: 1.5}}>
                    {excelProps.summary}
                </Text>

                <Button variant="light" color="blue" fullWidth style={{marginTop: 14}}
                        onClick={() => router.push("/excel/" + excelProps.id)}>
                    Află mai multe
                </Button>
            </Card>
        </div>
    );
}

